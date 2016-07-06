use rustc_serialize::json;
use chrono::{Duration, UTC, DateTime};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::vec::Vec;
use std::io::prelude::*;
use errors::{CliError, LalResult};


/// Representation of a docker volume mount for `.lalrc`
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Mount {
    /// File or folder to mount
    pub src: String,
    /// Location inside the container to mount it at
    pub dest: String,
    /// Whether or not to write protect the mount inside the container
    pub readonly: bool,
}

/// Static Artifactory locations to use
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct ArtifactoryConfig {
    /// Location of artifactory server
    pub server: String,
    /// Group to fetch artifacts from in artifactory
    pub group: String,
}

/// Representation of `lalrc`
#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Config {
    /// Configuration settings for Artifactory
    pub artifactory: ArtifactoryConfig,
    /// Cache directory for global and stashed builds
    pub cache: String,
    /// Docker container (potentially with tag) to use
    pub container: String,
    /// Time of last upgrade_check
    pub upgradeCheck: String,
    /// Extra volume mounts to be set for the container
    pub mounts: Vec<Mount>,
}

// Representation of a repo-wide overriding .lalrc
#[derive(RustcDecodable, Clone)]
struct PartialConfig {
    /// Overridden container to use for this repo
    container: Option<String>,
    /// Overridden mounts to use for this repo
    mounts: Option<Vec<Mount>>,
}
impl PartialConfig {
    fn read() -> LalResult<Option<PartialConfig>> {
        let cfg_path = Path::new(".lalrc");
        if !cfg_path.exists() {
            return Ok(None);
        }
        let mut f = try!(fs::File::open(&cfg_path));
        let mut cfg_str = String::new();
        try!(f.read_to_string(&mut cfg_str));
        let res: PartialConfig = try!(json::decode(&cfg_str));
        Ok(Some(res))
    }
}

impl Config {
    /// Initialize a Config with defaults
    ///
    /// This will locate you homedir, and set last update check 2 days in the past.
    /// Thus, with a blank default config, you will always trigger an upgrade check.
    pub fn new() -> LalResult<Config> {
        // unwrapping things that really must succeed here
        let home = env::home_dir().unwrap();
        let cachepath = Path::new(&home).join(".lal").join("cache");
        let cachedir = cachepath.as_path().to_str().unwrap();
        let time = UTC::now() - Duration::days(2);
        let artf = ArtifactoryConfig {
            server: "https://engci-maven-master.cisco.com/artifactory".to_string(),
            group: "CME-release".to_string(),
        };
        let mut mounts = vec![];
        // add default tools mount for media people if it exists on their machine
        let tools_mount = Path::new("/mnt/tools");
        if tools_mount.exists() {
            mounts.push(Mount {
                src: "/mnt/tools".into(),
                dest: "/tools".into(),
                readonly: true,
            })
        }
        Ok(Config {
            artifactory: artf,
            cache: cachedir.to_string(),
            container: "edonusdevelopers/centos_build:latest".to_string(),
            upgradeCheck: time.to_rfc3339(),
            mounts: mounts,
        })
    }
    /// Read and deserialize a Config from ~/.lal/lalrc
    pub fn read() -> LalResult<Config> {
        let home = env::home_dir().unwrap(); // crash if no $HOME
        let cfg_path = Path::new(&home).join(".lal/lalrc");
        if !cfg_path.exists() {
            return Err(CliError::MissingConfig);
        }
        let mut f = try!(fs::File::open(&cfg_path));
        let mut cfg_str = String::new();
        try!(f.read_to_string(&mut cfg_str));
        let mut res: Config = try!(json::decode(&cfg_str));
        let overrides = try!(PartialConfig::read());
        if overrides.is_some() {
            let partial = overrides.unwrap();
            if partial.container.is_some() {
                res.container = partial.container.unwrap();
            }
            if partial.mounts.is_some() {
                res.mounts = partial.mounts.unwrap();
            }
        }
        Ok(res)
    }
    /// Checks if it is time to perform an upgrade check
    pub fn upgrade_check_time(&self) -> bool {
        let last = self.upgradeCheck.parse::<DateTime<UTC>>().unwrap();
        let cutoff = UTC::now() - Duration::days(1);
        last < cutoff
    }
    /// Update the upgradeCheck time to avoid triggering it for another day
    pub fn performed_upgrade(&mut self) -> LalResult<()> {
        self.upgradeCheck = UTC::now().to_rfc3339();
        Ok(try!(self.write(true)))
    }
    /// Overwrite `~/.lal/lalrc` with serialized data from this struct
    pub fn write(&self, silent: bool) -> LalResult<()> {
        let home = env::home_dir().unwrap();
        let cfg_path = Path::new(&home).join(".lal").join("lalrc");

        let encoded = json::as_pretty_json(self);

        let mut f = try!(fs::File::create(&cfg_path));
        try!(write!(f, "{}\n", encoded));
        if silent {
            debug!("Wrote config {}: \n{}", cfg_path.display(), encoded);
        } else {
            info!("Wrote config {}: \n{}", cfg_path.display(), encoded);
        }
        Ok(())
    }
}


fn prompt(name: &str, default: String) -> String {
    use std::io::{self, Write};
    print!("Default {}: ({}) ", name, &default);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            if n > 1 {
                // more than just a newline character (which we strip)
                return (&input[0..n - 1]).to_string();
            }
        }
        Err(error) => println!("error: {}", error),
    }
    default
}

fn create_lal_dir() -> LalResult<PathBuf> {
    let home = env::home_dir().unwrap();
    let laldir = Path::new(&home).join(".lal");
    if !laldir.is_dir() {
        try!(fs::create_dir(&laldir));
    }
    Ok(laldir)
}

/// Create  `~/.lal/lalrc` interactively
///
/// This will prompt you interactively when setting `term_prompt`
/// Otherwise will just use the defaults.
///
/// A second boolean option to discard the output is supplied for tests.
pub fn configure(term_prompt: bool, save: bool, container: Option<&str>) -> LalResult<Config> {
    let _ = try!(create_lal_dir());
    let mut cfg = try!(Config::new());

    if term_prompt {
        // Prompt for values:
        cfg.artifactory.server = prompt("artifactory server", cfg.artifactory.server);
        cfg.artifactory.group = prompt("artifactory group", cfg.artifactory.group);
        cfg.cache = prompt("cache directory", cfg.cache);
        cfg.container = prompt("container", cfg.container);
    }
    // Tests to avoid depending on other containers
    if container.is_some() {
        cfg.container = container.unwrap().to_string();
    }

    if save {
        try!(cfg.write(false));
    }

    Ok(cfg.clone())
}
