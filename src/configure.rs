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

fn kernel_sanity() -> LalResult<()> {
    let req = Version { major: 4, minor: 4, patch: 0, pre: vec![], build: vec![] };
    let uname_output = Command::new("uname").arg("-r").output()?;
    let uname = String::from_utf8_lossy(&uname_output.stdout);
    match uname.trim().parse::<Version>() {
        Ok(ver) => {
            debug!("Found linux kernel version {}", ver);
            if ver < req {
                warn!("Your Linux kernel {} is very old", ver.to_string());
                warn!("A kernel >= {} is highly recommended on Linux systems", req.to_string())
            } else {
                debug!("Minimum kernel requirement of {} satisfied ({})", req.to_string(), ver.to_string());
            }
        }
        Err(e) => {
            // NB: Darwin would enter here..
            warn!("Failed to parse kernel version from `uname -r`: {}", e);
            warn!("Note that a kernel version of 4.4 is expected on linux");
        }
    }
    Ok(()) // don't block on this atm to not break OSX
}

fn docker_version_check() -> LalResult<()> {
    // docker-ce changes to different version scheme, but still semver >= 1.13
    let req = Version { major: 1, minor: 12, patch: 0, pre: vec![], build: vec![] };
    // NB: this is nicer: `docker version -f "{{ .Server.Version }}"`
    // but it doesn't work on the old versions we wnat to prevent..
    let dver_output = Command::new("docker").arg("--version").output()?;
    let dverstr = String::from_utf8_lossy(&dver_output.stdout);
    trace!("docker version string {}", dverstr);
    let dverary = dverstr.trim().split(" ").collect::<Vec<_>>();
    if dverary.len() < 3 {
        warn!("Failed to parse docker version: ({})", dverstr);
        return Ok(()); // assume it's a really weird docker
    }
    let mut dver = dverary[2].to_string(); // third entry is the semver version
    dver.pop(); // remove trailing comma (even if it goes, this parses)
    match dver.parse::<Version>() {
        Ok(ver) => {
            debug!("Found docker version {}", ver);
            if ver < req {
                warn!("Your docker version {} is very old", ver.to_string());
                warn!("A docker version >= {} is highly recommended", req.to_string())
            } else {
                debug!("Minimum docker requirement of {} satisfied ({})", req.to_string(), ver.to_string());
            }
        }
        Err(e) => {
            warn!("Failed to parse docker version from `docker --version`: {}", e);
            warn!("Note that a docker version >= 1.12 is expected");
        }
    }
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

    let sslcerts = Path::new("/etc/ssl/certs/ca-certificates.crt");
    if !sslcerts.exists() {
        warn!("Standard SSL certificates package missing");
        warn!("Please ensure you have the standard ca-certificates package");
        warn!("Alternatively set the SSL_CERT_FILE in you shell to prevent certificate errors");
        warn!("This is usually needed on OSX / CentOS");
    } else {
        trace!("Found valid SSL certificate bundle at {}", sslcerts.display());
    }
    // TODO: root id check

    for exe in ["docker", "tar", "touch", "id", "find", "mkdir", "chmod"].into_iter() {
        exists(exe)?;
    }
    docker_sanity()?;
    docker_version_check()?;
    kernel_sanity()?;

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
