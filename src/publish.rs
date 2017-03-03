use std::path::Path;
use std::fs::File;

// Need both the struct and the trait
use backend::{Artifactory, Backend};
use super::{LalResult, CliError, Lockfile};

/// Publish a release build to artifactory
///
/// Meant to be done after a `lal build -r <component>`
/// and requires artifactory publish credentials in the local `Config`.
pub fn publish(name: &str, backend: &Artifactory, env: &str) -> LalResult<()> {
    let artdir = Path::new("./ARTIFACT");
    let tarball = artdir.join(format!("{}.tar.gz", name));
    let lockfile = artdir.join("lockfile.json");
    if !artdir.is_dir() || !tarball.exists() {
        return Err(CliError::MissingReleaseBuild);
    }

    let lock = Lockfile::release_build()?;

    let version = lock.version
        .parse::<u32>()
        .map_err(|e| {
            error!("Release build not done --with-version=$BUILD_VERSION");
            debug!("Error: {}", e);
            CliError::MissingReleaseBuild
        })?;

    if lock.sha.is_none() {
        warn!("Release build not done --with-sha=$(git rev-parse HEAD)");
    }

    assert_eq!(env, lock.environment); // for now

    info!("Publishing {}={}", name, version);

    let tar_uri = format!("{}/{}/{}.tar.gz", name, version, name);
    let mut tarf = File::open(tarball)?;
    backend.upload_file(&tar_uri, &mut tarf)?;

    let mut lockf = File::open(lockfile)?;
    let lf_uri = format!("{}/{}/lockfile.json", name, version);
    backend.upload_file(&lf_uri, &mut lockf)?;

    Ok(())
}
