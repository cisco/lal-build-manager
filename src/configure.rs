use std::path::PathBuf;
use std::fs;

use super::{LalResult, Config, ConfigDefaults, config_dir};

fn create_lal_dir() -> LalResult<PathBuf> {
    let laldir = config_dir();
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
