use storage::CachedBackend;
use super::{LalResult, Manifest, CliError};

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
    components: Vec<String>,
    save: bool,
    savedev: bool,
    env: &str,
) -> LalResult<()> {
    debug!("Update specific deps: {:?}", components);

    let mut error = None;
    let mut updated = Vec::with_capacity(components.len());
    for comp in &components {
        info!("Fetch {} {}", env, comp);
        if comp.contains('=') {
            let pair: Vec<&str> = comp.split('=').collect();
            if let Ok(n) = pair[1].parse::<u32>() {
                if pair[0].to_lowercase() != pair[0] {
                    return Err(CliError::InvalidComponentName(pair[0].into()));
                }
                // standard fetch with an integer version
                match backend.unpack_published_component(pair[0], Some(n), env) {
                    Ok(c) => updated.push(c),
                    Err(e) => {
                        warn!("Failed to update {} ({})", pair[0], e);
                        error = Some(e);
                    }
                }
            } else {
                // fetch from stash - this does not go into `updated` it it succeeds
                // because we wont and cannot save stashed versions in the manifest
                let _ = backend.unpack_stashed_component(pair[0], pair[1]).map_err(|e| {
                    warn!("Failed to update {} from stash ({})", pair[0], e);
                    error = Some(e);
                });
            }
        } else {
            if &comp.to_lowercase() != comp {
                return Err(CliError::InvalidComponentName(comp.clone()));
            }
            // fetch without a specific version (latest)

            // First, since this potentially goes in the manifest
            // make sure the version is found for all supported environments:
            let ver = backend
                .get_latest_supported_versions(comp, manifest.supportedEnvironments.clone())?
                .into_iter()
                .max()
                .ok_or(CliError::NoIntersectedVersion(comp.clone()))?;
            info!("Fetch {} {}={}", env, comp, ver);

            match backend.unpack_published_component(comp, Some(ver), env) {
                Ok(c) => updated.push(c),
                Err(e) => {
                    warn!("Failed to update {} ({})", &comp, e);
                    error = Some(e);
                }
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
        let mut hmap = if save { mf.dependencies.clone() } else { mf.devDependencies.clone() };
        for c in &updated {
            debug!("Successfully updated {} at version {}", &c.name, c.version);
            if hmap.contains_key(&c.name) {
                let val = hmap.get_mut(&c.name).unwrap();
                if c.version < *val {
                    warn!("Downgrading {} from {} to {}", c.name, *val, c.version);
                } else if c.version > *val {
                    info!("Upgrading {} from {} to {}", c.name, *val, c.version);
                } else {
                    info!("Maintaining {} at version {}", c.name, c.version);
                }
                *val = c.version;
            } else {
                hmap.insert(c.name.clone(), c.version);
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
    update(manifest, backend, deps, save && !dev, save && dev, env)
}
