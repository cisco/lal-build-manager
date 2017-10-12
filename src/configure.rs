use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::process::Command;
use semver::Version;

use super::{LalResult, Config, ConfigDefaults, CliError, config_dir};

fn executable_on_path(exe: &str) -> LalResult<()> {
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
    let doutstr = String::from_utf8_lossy(&dinfo_output.stdout);
    if doutstr.contains("aufs") {
        warn!("Your storage driver is AUFS - this is known to have build issues");
        warn!("Please change your storage driver to overlay2 or devicemapper");
        warn!("Consult https://docs.docker.com/engine/userguide/storagedriver/ for info");
    }
    // TODO: Can grep for CPUs, RAM  if in the config perhaps?
    Ok(())
}

fn kernel_sanity() -> LalResult<()> {
    use semver::Identifier;
    // NB: ubuntu's use of linux kernel is not completely semver
    // the pre numbers does not indicate a prerelease, but rather fixes
    // thus 4.4.0-93 on ubuntu is semver LESS than semver 4.4.0
    // We thus restrict to be > 4.4.0-0-0 instead (>= number of pre-identifiers)
    let req = Version { major: 4, minor: 4, patch: 0,
        pre: vec![Identifier::Numeric(0), Identifier::Numeric(0)],
        build: vec![],
    };
    let uname_output = Command::new("uname").arg("-r").output()?;
    let uname = String::from_utf8_lossy(&uname_output.stdout);
    match uname.trim().parse::<Version>() {
        Ok(ver) => {
            debug!("Found linux kernel version {}", ver);
            trace!("found major {} minor {} patch {} - prelen {}", ver.major, ver.minor, ver.patch, ver.pre.len());
            trace!("req major {} minor {} patch {} - prelen {}", req.major, req.minor, req.patch, req.pre.len());
            if ver >= req {
                debug!("Minimum kernel requirement of {} satisfied ({})", req.to_string(), ver.to_string());
            } else {
                warn!("Your Linux kernel {} is very old", ver.to_string());
                warn!("A kernel >= {} is highly recommended on Linux systems", req.to_string())
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

fn ssl_cert_sanity() -> LalResult<()> {
    // SSL_CERT_FILE is overridden by main.rs for the benefit of musl built openssl
    // so be a little careful if it was actually set by user, or by main.rs
    let default_cert = "/etc/ssl/certs/ca-certificates.crt";
    let sslpath = env::var_os("SSL_CERT_FILE").unwrap_or_else(|| default_cert.into());
    let sslcerts = Path::new(&sslpath);
    trace!("Lookind for SSL certificates at {}", sslcerts.display());
    if !sslcerts.exists() {
        warn!("CA certificates missing - you will encounter ssl errors");
        if &sslpath == default_cert {
            warn!("Please ensure you have the standard ca-certificates package");
            warn!("Alternatively set the SSL_CERT_FILE in you shell to prevent certificate errors");
            warn!("This is usually needed on OSX / older unsupported linux distos");
        } else {
            warn!("You are overriding SSL_CERT_FILE to point to a file that does not exist");
            warn!("Try /etc/ssl/certs/ca-certificates.crt on ubuntu");
            warn!("Try /usr/local/etc/openssl/cert.pem on darwin");
        }
        Err(CliError::MissingSslCerts(format!("{}", sslcerts.display())))
    } else {
        trace!("Found valid SSL certificate bundle at {}", sslcerts.display());
        Ok(())
    }
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

fn non_root_sanity() -> LalResult<()> {
    let uid_output = Command::new("id").arg("-u").output()?;
    let uid_str = String::from_utf8_lossy(&uid_output.stdout);
    let uid = uid_str.trim().parse::<u32>().unwrap(); // trust `id -u` is sane

    if uid == 0 {
        warn!("Running lal as root user not allowed");
        warn!("Builds remap your user id to a corresponding one inside a build environment");
        warn!("This is at the moment incompatible with the root user");
        warn!("Try again without sudo, or if you are root, create a proper build user");
        Err(CliError::UnmappableRootUser)
    } else {
        Ok(())
    }
}

fn create_lal_dir() -> LalResult<PathBuf> {
    let laldir = config_dir();
    if !laldir.is_dir() {
        fs::create_dir(&laldir)?;
    }
    let histfile = Path::new(&laldir).join("history");
    if !histfile.exists() {
        fs::File::create(histfile)?;
    }
    Ok(laldir)
}

/// Create  `~/.lal/config` with defaults
///
/// A boolean option to discard the output is supplied for tests.
/// A defaults file must be supplied to seed the new config with defined environments
pub fn configure(save: bool, interactive: bool, defaults: &str) -> LalResult<Config> {
    let _ = create_lal_dir()?;
    // TODO: root id check

    for exe in ["docker", "tar", "touch", "id", "find", "mkdir", "chmod", "uname"].into_iter() {
        executable_on_path(exe)?;
    }
    docker_sanity()?;
    docker_version_check()?;
    kernel_sanity()?;
    ssl_cert_sanity()?;
    non_root_sanity()?;

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
