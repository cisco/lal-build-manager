use std::path::{Path, PathBuf};
use std::fs;
use std::env;

use super::{LalResult, Config, ConfigDefaults};

/// Helper to print the configured environments from the config
pub fn env_list(cfg: &Config) -> LalResult<()> {
    for k in cfg.environments.keys() {
        println!("{}", k);
    }
    Ok(())
}

fn create_lal_dir() -> LalResult<PathBuf> {
    let home = env::home_dir().unwrap();
    let laldir = Path::new(&home).join(".lal");
    if !laldir.is_dir() {
        fs::create_dir(&laldir)?;
    }
    Ok(laldir)
}

/// Create  `~/.lal/config` with defaults
///
/// A boolean option to discard the output is supplied for tests.
/// A defaults file must be supplied to seed the new config with defined environments
pub fn configure(save: bool, interactive: bool, defaults: &str) -> LalResult<Config> {
    let _ = create_lal_dir()?;

    let mut cfg = Config::new(ConfigDefaults::read(defaults)?);
    cfg.interactive = interactive; // need to override default for tests
    if save {
        cfg.write(false)?;
    }
    Ok(cfg)
}
