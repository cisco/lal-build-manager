use std::fs;

use semver::Version;
use util::artifactory::find_latest_lal_version;
use install::download_to_path;
use {LalResult, Config};

/// Upgrade binary installations of lal into a specified prefix
///
/// This will perform the installation regardless of what the current running lal is.
/// If you are running a source install, this may not be what you want,
/// but if you are running prebuilts, this is the fastest way to upgrade.
pub fn upgrade_binary(cfg: Config, version: Option<&str>, prefix_: Option<&str>) -> LalResult<()> {
    use tar::Archive;
    use flate2::read::GzDecoder;

    debug!("binary install");
    let install_version = match version {
        Some(v) => Version::parse(v).unwrap(), // TODO: try!
        None => try!(find_latest_lal_version()),
    };

    let prefix = prefix_.unwrap_or("/usr/local");
    info!("Installing to {}", prefix);
    let uri = format!("{}/lal/{}/lal.tar", cfg.artifactory, install_version);
    // TODO: will this even work if we're hotswapping the binary we are using!?

    let tarname = "./lal.tar";
    try!(download_to_path(&uri, tarname));

    debug!("Unpacking tarball {}", tarname);
    let data = try!(fs::File::open(&tarname));
    let decompressed = try!(GzDecoder::new(data)); // decoder reads data
    let mut archive = Archive::new(decompressed); // Archive reads decoded

    try!(fs::create_dir_all(&prefix));
    try!(archive.unpack(&prefix));

    info!("lal {} successfully installed", install_version);
    info!("Run `which lal` to ensure it comes from {}", prefix);
    Ok(())
}

/// Check for new versions of lal on artifactory
///
/// This will just query for the latest version, and not install anything.
/// If a newer version found (> in semver), then this is logged depending on mode.
/// If run as part of the automatic update check, then it's silent.
pub fn upgrade_check(silent: bool) -> LalResult<()> {
    let latest = try!(find_latest_lal_version());
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    if latest > current {
        // New version found - always full output now
        info!("A new version of lal is available: {}\n", latest);

        // Source install - just tell the user what to do regardless of dry_run:
        info!("If your version is compiled from source:");
        info!(" - `git pull` and `cargo build --release` in the source checkout");
        info!("If your version is fetched prebuilt:");
        info!(" - `lal upgrade --binary` (maybe supply --prefix=destination)");
    } else {
        // No new version
        if silent {
            debug!("You are running the latest version of lal");
        } else {
            info!("You are running the latest version of lal");
        }
    }
    Ok(())
}
