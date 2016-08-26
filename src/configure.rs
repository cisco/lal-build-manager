use rustc_serialize::json;
use chrono::{Duration, UTC, DateTime};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::vec::Vec;
use std::io::prelude::*;
use std::collections::BTreeMap;
use errors::{CliError, LalResult};
use super::Container;

// helper
fn lal_dir() -> PathBuf {
    // unwrapping things that really must succeed here
    let home = env::home_dir().unwrap();
    Path::new(&home).join(".lal")
}


/// Docker volume mount representation
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Mount {
    /// File or folder to mount
    pub src: String,
    /// Location inside the container to mount it at
    pub dest: String,
    /// Whether or not to write protect the mount inside the container
    pub readonly: bool,
}

/// Static Artifactory locations
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Artifactory {
    /// Location of artifactory API querying server
    pub server: String,
    /// Group parameter to use for API queries
    pub group: String,
    /// Location of artifactory virtual group for downloads
    pub vgroup: String,
}

/// Representation of `~/.lal/config`
#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Config {
    /// Configuration settings for Artifactory
    pub artifactory: Artifactory,
    /// Cache directory for global and stashed builds
    pub cache: String,
    /// Environments shorthands that are allowed and their full meaning
    pub environments: BTreeMap<String, Container>,
    /// Time of last upgrade_check
    pub upgradeCheck: String,
    /// Extra volume mounts to be set for the container
    pub mounts: Vec<Mount>,
}

/// Edonusdevelopers default artifactory server
impl Default for Artifactory {
    fn default() -> Self {
        Artifactory {
            server: "https://engci-maven-master.cisco.com/artifactory".into(),
            group: "CME-release".into(),
            vgroup: "https://engci-maven.cisco.com/artifactory/CME-group".into(),
        }
    }
}

/// Edonusdevelopers default Config
///
/// This will locate you homedir, and set last update check 2 days in the past.
/// Thus, with a blank default config, you will always trigger an upgrade check.
impl Default for Config {
    fn default() -> Self {
        let cachepath = lal_dir().join("cache");
        let cachedir = cachepath.as_path().to_str().unwrap();
        // edonusdevelopers default C++ containers
        let mut envs = BTreeMap::new();
        envs.insert("centos".into(),
                    Container {
                        name: "edonusdevelopers/centos_build".into(),
                        tag: "latest".into(),
                    });
        envs.insert("xenial".into(),
                    Container {
                        name: "edonusdevelopers/build_xenial".into(),
                        tag: "latest".into(),
                    });
        envs.insert("rust".into(),
                    Container {
                        name: "edonusdevelopers/muslrust".into(),
                        tag: "latest".into(),
                    });
        envs.insert("transcoder".into(),
                    Container {
                        name: "edonusdevelopers/mygdon-transcoder".into(),
                        tag: "latest".into(),
                    });
        // last update time
        let time = UTC::now() - Duration::days(2);
        // common edonusdevelopers mounts
        let mut mounts = vec![];
        let tools_mount = Path::new("/mnt/tools");
        if tools_mount.exists() {
            mounts.push(Mount {
                src: "/mnt/tools".into(),
                dest: "/tools".into(),
                readonly: true,
            })
        }
        let files_mount = Path::new("/mnt/build-files");
        if files_mount.exists() {
            mounts.push(Mount {
                src: "/mnt/build-files".into(),
                dest: "/build-files".into(),
                readonly: true,
            })
        }
        Config {
            cache: cachedir.into(),
            mounts: mounts,
            upgradeCheck: time.to_rfc3339(),
            environments: envs,
            artifactory: Artifactory::default(),
        }
    }
}

impl Config {
    /// Initialize a Config with defaults
    pub fn new() -> Config {
        Default::default()
    }
    /// Read and deserialize a Config from ~/.lal/config
    pub fn read() -> LalResult<Config> {
        let cfg_path = lal_dir().join("config");
        if !cfg_path.exists() {
            return Err(CliError::MissingConfig);
        }
        let mut f = try!(fs::File::open(&cfg_path));
        let mut cfg_str = String::new();
        try!(f.read_to_string(&mut cfg_str));
        let res: Config = try!(json::decode(&cfg_str));
        if res.environments.contains_key("default") {
            return Err(CliError::InvalidEnvironment);
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
    /// Overwrite `~/.lal/config` with serialized data from this struct
    pub fn write(&self, silent: bool) -> LalResult<()> {
        let cfg_path = lal_dir().join("config");

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

    /// Resolve an arbitrary container shorthand
    pub fn get_container(&self, env: Option<String>) -> LalResult<Container> {
        let env_ = if env.is_none() { "centos".into() } else { env.unwrap() };
        if let Some(container) = self.environments.get(&env_) {
            return Ok(container.clone());
        }
        Err(CliError::MissingEnvironment(env_))
    }
}

fn create_lal_dir() -> LalResult<PathBuf> {
    let home = env::home_dir().unwrap();
    let laldir = Path::new(&home).join(".lal");
    if !laldir.is_dir() {
        try!(fs::create_dir(&laldir));
    }
    Ok(laldir)
}

/// Create  `~/.lal/config` with defaults
///
/// A boolean option to discard the output is supplied for tests.
pub fn configure(save: bool) -> LalResult<Config> {
    let _ = try!(create_lal_dir());
    let cfg = Config::new();
    if save {
        try!(cfg.write(false));
    }
    Ok(cfg)
}
