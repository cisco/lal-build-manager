use std::path::Path;

// Need both the struct and the trait
use storage::Backend;
use super::{LalResult, CliError, Lockfile};

/// Publish a release build to the storage backend
///
/// Meant to be done after a `lal build -r <component>`
/// and requires publish credentials in the local `Config`.
pub fn publish<T: Backend + ?Sized>(name: &str, backend: &T) -> LalResult<()> {
    let artdir = Path::new("./ARTIFACT");
    let tarball = artdir.join(format!("{}.tar.gz", name));
    if !artdir.is_dir() || !tarball.exists() {
        warn!("Missing: {}", tarball.display());
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

    // always publish to the environment in the lockfile
    let env = lock.environment;

    info!("Publishing {}={} to {}", name, version, env);
    backend.publish_artifact(name, version, &env)?;

    Ok(())
}
