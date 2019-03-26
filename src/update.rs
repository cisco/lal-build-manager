use super::{CliError, Coordinates, LalResult, Manifest, TwoDCoordinates};
use crate::channel::{parse_coords, Channel};
use crate::storage::CachedBackend;

/// Update specific dependencies outside the manifest
///
/// Multiple "components=version" strings can be supplied, where the version is optional.
/// If no version is supplied, latest is fetched.
///
/// If installation was successful, the fetched tarballs are unpacked into `./INPUT`.
/// If one `save` or `savedev` was set, the fetched versions are also updated in the
/// manifest. This provides an easy way to not have to deal with strict JSON manually.
pub fn update<T: CachedBackend + ?Sized>(
    manifest: &Manifest,
    backend: &T,
    components: &[String],
    save: bool,
    savedev: bool,
    env: &str,
) -> LalResult<()> {
    debug!("Update specific deps: {:?}", components);

    // Verify component names.
    components
        .iter()
        .map(|c| c.split('=').collect::<Vec<_>>()[0])
        .try_for_each(|c| {
            if c.to_lowercase() == c {
                Ok(())
            } else {
                Err(CliError::InvalidComponentName(c.to_string()))
            }
        })?;

    let mut error = None;
    let mut updated = Vec::with_capacity(components.len());
    let own_channel = Channel::from_option(&manifest.channel);
    for comp in components {
        info!("Fetch {} {}", env, comp);
        let comp_vec = comp.split('=').collect::<Vec<_>>();
        let comp = comp_vec[0];

        if comp_vec.len() == 1 {
            // fetch without a specific version (latest)

            // First, identify the current channel.
            let coords = if let Some(coords) = manifest.dependencies.get(comp) {
                Some(coords)
            } else if let Some(coords) = manifest.devDependencies.get(comp) {
                Some(coords)
            } else {
                None
            };
            let current_channel = if let Some(coords) = coords {
                match coords {
                    Coordinates::OneD(_) => None,
                    Coordinates::TwoD(v) => Some(&v.channel),
                }
            } else {
                None
            };
            let current_channel = Channel::from_option(&current_channel);
            debug!("Current channel: {}", current_channel);

            // Second, since this potentially goes in the manifest
            // make sure the version is found for all supported environments:
            let ver = *backend
                .get_latest_supported_versions(
                    comp,
                    manifest.supportedEnvironments.clone(),
                    &current_channel,
                )?
                .last()
                .ok_or_else(|| CliError::NoIntersectedVersion(comp.to_string()))?;
            info!("Fetch {} {}={}{}", env, comp, current_channel.version_string(), ver);

            match backend.unpack_published_component(comp, Some(ver), env, &current_channel) {
                Ok(c) => updated.push(c),
                Err(e) => {
                    warn!("Failed to update {} ({})", &comp, e);
                    error = Some(e);
                }
            }
        } else {
            // Put the original `=` signs back in place.
            let coords = comp_vec.iter().skip(1).cloned().collect::<Vec<_>>().join("=");
            let (version, channel) = parse_coords(&coords);

            if version.is_some() || channel.is_some() {
                // We have at least one of a version or a channel.
                let channel = Channel::from_option(&channel);

                // Verify that the channel is valid.
                if let Err(e) = channel.verify() {
                    warn!("Invalid channel {}", channel);
                    error = Some(e);
                    continue;
                }
                if let Err(e) = own_channel.contains(&channel) {
                    warn!("Failed to update {} ({})", &comp, e);
                    error = Some(e);
                    continue;
                }

                match backend.unpack_published_component(comp, version, env, &channel) {
                    Ok(c) => updated.push(c),
                    Err(e) => {
                        warn!("Failed to update {} ({})", comp, e);
                        error = Some(e);
                    }
                }
            } else {
                // fetch from stash - this does not go into `updated` it it succeeds
                // because we won't and cannot save stashed versions in the manifest
                let _ = backend
                    .unpack_stashed_component(comp, &coords)
                    .map_err(|e| {
                        warn!("Failed to update {} from stash ({})", comp, e);
                        error = Some(e);
                    });
            }
        }
    }
    if let Some(e) = error {
        return Err(e);
    }

    // Update manifest if saving in any way
    if save || savedev {
        let mut mf = manifest.clone();
        // find reference to correct list
        let mut hmap = if save {
            mf.dependencies.clone()
        } else {
            mf.devDependencies.clone()
        };
        for c in &updated {
            debug!(
                "Successfully updated {} on channel {} to version {}",
                c.name, c.channel, c.version
            );
            let new_coords = Coordinates::TwoD(TwoDCoordinates {
                version: c.version,
                channel: c.channel.to_string(),
            });
            if hmap.contains_key(&c.name) {
                let val = hmap.get_mut(&c.name).unwrap();
                let (version, channel) = match val {
                    Coordinates::OneD(v) => (*v, None),
                    Coordinates::TwoD(v) => (v.version, Some(&v.channel)),
                };
                // Only print information about changes in versions if the channel has
                // remained constant. Otherwise the version numbers cannot be compared.
                let channel = Channel::from_option(&channel);
                if c.channel != channel {
                    info!("Changing from channel {} to channel {}", channel, c.channel)
                } else if c.version < version {
                    warn!("Downgrading {} from {} to {}", c.name, version, c.version);
                } else if c.version > version {
                    info!("Upgrading {} from {} to {}", c.name, version, c.version);
                } else {
                    info!("Maintaining {} at version {}", c.name, c.version);
                }
                *val = new_coords;
            } else {
                hmap.insert(c.name.clone(), new_coords);
            }
        }
        if save {
            mf.dependencies = hmap;
        } else {
            mf.devDependencies = hmap;
        }
        mf.write()?;
    }
    Ok(())
}

/// Wrapper around update that updates all components
///
/// This will pass all dependencies or devDependencies to update.
/// If the save flag is set, then the manifest will be updated correctly.
/// I.e. dev updates will update only the dev portions of the manifest.
pub fn update_all<T: CachedBackend + ?Sized>(
    manifest: &Manifest,
    backend: &T,
    save: bool,
    dev: bool,
    env: &str,
) -> LalResult<()> {
    let deps: Vec<String> = if dev {
        manifest.devDependencies.keys().cloned().collect()
    } else {
        manifest.dependencies.keys().cloned().collect()
    };
    update(manifest, backend, &deps, save && !dev, save && dev, env)
}
