use semver::Version;
use super::{LalResult, Backend, Artifactory};

/// Check for new versions of lal on artifactory
///
/// This will just query for the latest version, and not install anything.
/// If a newer version found (> in semver), then this is logged depending on mode.
/// If run as part of the automatic update check, then it's silent.
pub fn upgrade_check(backend: &Artifactory, silent: bool) -> LalResult<bool> {
    let latest = backend.get_latest_lal_version()?;
    let cfg = backend.config.clone();
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    if latest > current {
        // New version found - always full output now
        info!("A new version of lal is available: {}", latest);
        trace!("You are running {}", current);
        println!("");

        // Source install - just tell the user what to do regardless of dry_run:
        info!("If your version is compiled from source:");
        info!(" - `git pull && cargo build --release` in the source checkout");
        info!("If your version is prebuilt:");
        info!(" - `curl {}/{}/lal/latest/lal.tar | tar xz -C /usr/local`",
              cfg.slave,
              cfg.vgroup);
    } else if silent {
        debug!("You are running the latest version of lal");
    } else {
        info!("You are running the latest version of lal");
    }
    Ok(latest > current)
}
