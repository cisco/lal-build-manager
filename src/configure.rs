use std::path::PathBuf;
use std::fs;
use std::process::Command;

use super::{LalResult, Config, ConfigDefaults, CliError, config_dir};

fn exists(exe: &str) -> LalResult<()> {
    trace!("Verifying executable {}", exe);
    let s = Command::new("which")
                    .arg(exe)
                    .output()?;
    if !s.status.success() {
        debug!("Failed to find {}: {}", exe, String::from_utf8_lossy(&s.stderr).trim());
        return Err(CliError::ExecutableMissing(exe.into()));
    };
    debug!("Found {} at {}", exe, String::from_utf8_lossy(&s.stdout).trim());
    Ok(())
}

fn docker_sanity() -> LalResult<()> {
    let dinfo_output = Command::new("docker").arg("info").output()?;
    let _ = String::from_utf8_lossy(&dinfo_output.stdout);
    // TODO: Can grep for CPUs, RAM, storage driver, if in the config
    // TODO: check
    Ok(())
}


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

    for exe in ["docker", "tar", "touch", "id", "find", "mkdir", "chmod"].into_iter() {
        exists(exe)?;
    }
    docker_sanity()?;

    let mut cfg = Config::new(ConfigDefaults::read(defaults)?);
    cfg.interactive = interactive; // need to override default for tests
    if save {
        cfg.write(false)?;
    }
    Ok(cfg)
}
