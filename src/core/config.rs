use chrono::UTC;
use serde_json;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::vec::Vec;

use super::{CliError, Container, LalResult};
use crate::storage::BackendConfiguration;

fn find_home_dir() -> PathBuf {
    // Either we have LAL_CONFIG_HOME evar, or HOME
    if let Ok(lh) = env::var("LAL_CONFIG_HOME") {
        Path::new(&lh).to_owned()
    } else {
        dirs::home_dir().unwrap()
    }
}

/// Master override for where the .lal config lives
pub fn config_dir() -> PathBuf {
    let home = find_home_dir();
    Path::new(&home).join(".lal")
}

/// Docker volume mount representation
#[derive(Serialize, Deserialize, Clone)]
pub struct Mount {
    /// File or folder to mount
    pub src: String,
    /// Location inside the container to mount it at
    pub dest: String,
    /// Whether or not to write protect the mount inside the container
    pub readonly: bool,
}

/// Representation of `~/.lal/config`
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// Configuration settings for the `Backend`
    pub backend: BackendConfiguration,
    /// Cache directory for global and stashed builds
    pub cache: String,
    /// Environments shorthands that are allowed and their full meaning
    pub environments: BTreeMap<String, Container>,
    /// Time of last upgrade
    pub lastUpgrade: String,
    /// Whether to perform automatic upgrade
    pub autoupgrade: bool,
    /// Extra volume mounts to be set for the container
    pub mounts: Vec<Mount>,
    /// Force inteactive shells
    pub interactive: bool,
    /// Minimum version restriction of lal enforced by this config
    pub minimum_lal: Option<String>,
}

/// Representation of a configuration defaults file
///
/// This file is being used to generate the config when using `lal configure`
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ConfigDefaults {
    /// Configuration settings for the `Backend`
    pub backend: BackendConfiguration,
    /// Environments shorthands that are allowed and their full meaning
    pub environments: BTreeMap<String, Container>,
    /// Extra volume mounts to be set for the container
    pub mounts: Vec<Mount>,
    /// Optional minimum version restriction of lal
    pub minimum_lal: Option<String>,
}

impl ConfigDefaults {
    /// Open and deserialize a defaults file
    pub fn read(file: &str) -> LalResult<Self> {
        let pth = Path::new(file);
        if !pth.exists() {
            // Panics only if the current directory does not exist or the user does not have permissions to see the current directory.
            error!(
                "No such defaults file '{}' found from path '{:#?}",
                file,
                std::env::current_dir().unwrap()
            ); // file open will fail below
        }
        let mut f = fs::File::open(&pth)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let defaults: Self = serde_json::from_str(&data)?;
        Ok(defaults)
    }
}

fn check_mount(name: &str) -> LalResult<String> {
    // See if it's a path first:
    let home = find_home_dir();
    let src = name.to_string().replace("~", &home.to_string_lossy());
    let mount_path = Path::new(&src);
    if mount_path.exists() {
        debug!("Configuring existing mount {}", &src);
        return Ok(src.clone()); // pass along the real source
    }

    if name.contains('/') {
        warn!("Discarding missing mount {}", src);
        Err(CliError::MissingMount(src.clone()))
    } else {
        // Otherwise, if it does not contain a slash
        use std::process::Command;
        let volume_output = Command::new("docker")
            .args(vec!["volume", "ls", "-q"])
            .output()?;
        let volstr = String::from_utf8_lossy(&volume_output.stdout);
        // If it exists, do nothing:
        if volstr.contains(name) {
            debug!("Configuring existing volume {}", name);
            return Ok(name.into());
        }
        // Otherwise warn
        warn!("Discarding missing docker volume {}", name);
        Err(CliError::MissingMount(name.into()))
    }
}

impl Config {
    /// Initialize a Config with ConfigDefaults
    ///
    /// This will locate you homedir, and set last update check 2 days in the past.
    /// Thus, with a blank default config, you will always trigger an upgrade check.
    pub fn new(defaults: ConfigDefaults) -> Self {
        let cachepath = config_dir().join("cache");
        let cachedir = cachepath.as_path().to_str().unwrap();

        // reset last update time
        let time = UTC::now();

        // scan default mounts
        let mut mounts = vec![];
        for mount in defaults.mounts {
            // Check src for pathiness or prepare a docker volume
            match check_mount(&mount.src) {
                Ok(src) => {
                    let mut mountnew = mount.clone();
                    mountnew.src = src; // update potentially mapped source
                    mounts.push(mountnew);
                }
                Err(e) => debug!("Ignoring mount check error {}", e),
            }
        }

        Self {
            cache: cachedir.into(),
            mounts, // the filtered defaults
            lastUpgrade: time.to_rfc3339(),
            autoupgrade: cfg!(feature = "upgrade"),
            environments: defaults.environments,
            backend: defaults.backend,
            minimum_lal: defaults.minimum_lal,
            interactive: true,
        }
    }

    /// Read and deserialize a Config from ~/.lal/config
    pub fn read() -> LalResult<Self> {
        let cfg_path = config_dir().join("config");
        if !cfg_path.exists() {
            return Err(CliError::MissingConfig);
        }
        let mut f = fs::File::open(&cfg_path)?;
        let mut cfg_str = String::new();
        f.read_to_string(&mut cfg_str)?;
        let res: Self = serde_json::from_str(&cfg_str)?;
        Ok(res)
    }

    /// Checks if it is time to perform an upgrade check
    #[cfg(feature = "upgrade")]
    pub fn upgrade_check_time(&self) -> bool {
        use chrono::{DateTime, Duration};
        let last = self.lastUpgrade.parse::<DateTime<UTC>>().unwrap();
        let cutoff = UTC::now() - Duration::days(1);
        last < cutoff
    }
    /// Update the lastUpgrade time to avoid triggering it for another day
    #[cfg(feature = "upgrade")]
    pub fn performed_upgrade(&mut self) -> LalResult<()> {
        self.lastUpgrade = UTC::now().to_rfc3339();
        self.write(true)
    }

    /// Overwrite `~/.lal/config` with serialized data from this struct
    pub fn write(&self, silent: bool) -> LalResult<()> {
        let cfg_path = config_dir().join("config");
        let encoded = serde_json::to_string_pretty(self)?;

        let mut f = fs::File::create(&cfg_path)?;
        writeln!(f, "{}", encoded)?;
        if !silent {
            info!("Wrote config to {}", cfg_path.display());
        }
        debug!("Wrote config \n{}", encoded);
        Ok(())
    }

    /// Resolve an arbitrary container shorthand
    pub fn get_container(&self, env: String) -> LalResult<Container> {
        if let Some(container) = self.environments.get(&env) {
            return Ok(container.clone());
        }
        Err(CliError::MissingEnvironment(env))
    }
}
