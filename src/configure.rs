use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use semver::Version;

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

fn lal_version_check(minlal: &str) -> LalResult<()> {
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let req = Version::parse(minlal).unwrap();
    if current < req {
        Err(CliError::OutdatedLal(current.to_string(), req.to_string()))
    } else {
        debug!("Minimum lal requirement of {} satisfied ({})", req.to_string(), current.to_string());
        Ok(())
    }
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

    let sslcerts = Path::new("/etc/ssl/certs/ca-certificates.crt");
    if !sslcerts.exists() {
        warn!("Standard SSL certificates package missing");
        warn!("Please ensure you have the standard ca-certificates package");
        warn!("Alternatively set the SSL_CERT_FILE in you shell to prevent certificate errors");
        warn!("This is usually needed on OSX / CentOS");
    } else {
        trace!("Found valid SSL certificate bundle at {}", sslcerts.display());
    }

    let def = ConfigDefaults::read(defaults)?;

    // Enforce minimum_lal version check here if it's set in the defaults file
    if let Some(minlal) = def.minimum_lal.clone() {
        lal_version_check(&minlal)?;
    }

    let mut cfg = Config::new(def);
    cfg.interactive = interactive; // need to override default for tests
    if save {
        cfg.write(false)?;
    }
    Ok(cfg)
}
