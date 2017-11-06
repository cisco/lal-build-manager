use std::fs;
use std::path::Path;

use storage::CachedBackend;
use super::{LalResult, CliError};

/// Export a specific component from the storage backend
pub fn export<T: CachedBackend + ?Sized>(
    backend: &T,
    comp: &str,
    output: Option<&str>,
    _env: Option<&str>,
) -> LalResult<()> {
    let env = match _env {
        None => {
            error!("export is no longer allowed without an explicit environment");
            return Err(CliError::EnvironmentUnspecified)
        },
        Some(e) => e
    };

    if comp.to_lowercase() != comp {
        return Err(CliError::InvalidComponentName(comp.into()));
    }

    let dir = output.unwrap_or(".");
    info!("Export {} {} to {}", env, comp, dir);

    let mut component_name = comp; // this is only correct if no =version suffix
    let tarname = if comp.contains('=') {
        let pair: Vec<&str> = comp.split('=').collect();
        if let Ok(n) = pair[1].parse::<u32>() {
            // standard fetch with an integer version
            component_name = pair[0]; // save so we have sensible tarball names
            backend.retrieve_published_component(pair[0], Some(n), env)?.0
        } else {
            // string version -> stash
            component_name = pair[0]; // save so we have sensible tarball names
            backend.retrieve_stashed_component(pair[0], pair[1])?
        }
    } else {
        // fetch without a specific version (latest)
        backend.retrieve_published_component(comp, None, env)?.0
    };

    let dest = Path::new(dir).join(format!("{}.tar.gz", component_name));
    debug!("Copying {:?} to {:?}", tarname, dest);

    fs::copy(tarname, dest)?;
    Ok(())
}
