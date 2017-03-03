use std::fs;
use std::path::Path;

use backend::{self, Artifactory};
use super::LalResult;

/// Export a specific component from artifactory
pub fn export(backend: &Artifactory,
              comp: &str,
              output: Option<&str>,
              env: Option<&str>)
              -> LalResult<()> {
    let dir = output.unwrap_or(".");

    info!("Export {} {} to {}", env.unwrap_or("global"), comp, dir);

    let mut component_name = comp; // this is only correct if no =version suffix
    let tarname = if comp.contains('=') {
        let pair: Vec<&str> = comp.split('=').collect();
        if let Ok(n) = pair[1].parse::<u32>() {
            // standard fetch with an integer version
            component_name = pair[0]; // save so we have sensible tarball names
            backend::fetch_via_artifactory(backend, pair[0], Some(n), env)?.0
        } else {
            // string version -> stash
            component_name = pair[0]; // save so we have sensible tarball names
            backend::get_path_to_stashed_component(backend, pair[0], pair[1])?
        }
    } else {
        // fetch without a specific version (latest)
        backend::fetch_via_artifactory(backend, comp, None, env)?.0
    };

    let dest = Path::new(dir).join(format!("{}.tar.gz", component_name));
    debug!("Copying {:?} to {:?}", tarname, dest);

    fs::copy(tarname, dest)?;
    Ok(())
}
