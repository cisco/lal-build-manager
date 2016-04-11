use semver::Version;
use util::artifactory::find_latest_lal_version;
use {LalResult, CliError, Config};

/// Check for updated versions of the lal binary on artifactory
pub fn upgrade(cfg: Config, check_only: bool) -> LalResult<()> {
    let latest = try!(find_latest_lal_version());
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    if latest > current {
        info!("A new version of lal is available: {}", latest)
    } else {
        trace!("You are running the latest version of lal");
    }
    if !check_only {
        unimplemented!();
    }
    Ok(())
}
