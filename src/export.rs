use std::fs;
use std::path::Path;

use super::{CliError, LalResult};
use crate::channel::{parse_coords, Channel};
use crate::storage::CachedBackend;

/// Export a specific component from the storage backend
pub fn export<T: CachedBackend + ?Sized>(
    backend: &T,
    comp: &str,
    output: Option<&str>,
    env: Option<&str>,
) -> LalResult<()> {
    let env = match env {
        None => {
            error!("export is no longer allowed without an explicit environment");
            return Err(CliError::EnvironmentUnspecified);
        }
        Some(e) => e,
    };

    if comp.to_lowercase() != comp {
        return Err(CliError::InvalidComponentName(comp.into()));
    }

    let dir = output.unwrap_or(".");
    info!("Export {} {} to {}", env, comp, dir);

    let comp_vec = comp.split('=').collect::<Vec<_>>();
    let comp_name = comp_vec[0];
    let tarname = if comp_vec.len() > 1 {
        // Put the original `=` signs back in place.
        let coords = comp_vec.iter().skip(1).cloned().collect::<Vec<_>>().join("=");
        let (version, channel) = parse_coords(&coords);
        let channel = match channel {
            Some(ch) => ch,
            None => Channel::default(),
        };

        // Verify the channel is valid.
        channel.verify()?;

        if version.is_some() {
            // standard fetch with an integer version
            backend
                .retrieve_published_component(comp_name, version, env, &channel)?
                .0
        } else {
            backend.retrieve_stashed_component(comp_name, &coords)?
        }
    } else {
        // fetch without a specific version and channel (latest, and default)
        backend
            .retrieve_published_component(comp_name, None, env, &Channel::default())?
            .0
    };

    let dest = Path::new(dir).join(format!("{}.tar.gz", comp_name));
    debug!("Copying {:?} to {:?}", tarname, dest);

    fs::copy(tarname, dest)?;
    Ok(())
}
