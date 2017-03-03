use backend::{self, Artifactory};
use super::{LalResult, Manifest};


/// Update specific dependencies outside the manifest
///
/// Multiple "components=version" strings can be supplied, where the version is optional.
/// If no version is supplied, latest is fetched.
///
/// If installation was successful, the fetched tarballs are unpacked into `./INPUT`.
/// If one `save` or `savedev` was set, the fetched versions are also updated in the
/// manifest. This provides an easy way to not have to deal with strict JSON manually.
pub fn update(manifest: &Manifest,
              backend: &Artifactory,
              components: Vec<String>,
              save: bool,
              savedev: bool,
              env: &str)
              -> LalResult<()> {
    debug!("Update specific deps: {:?}", components);

    let mut error = None;
    let mut updated = Vec::with_capacity(components.len());
    for comp in &components {
        info!("Fetch {} {}", env, comp);
        if comp.contains('=') {
            let pair: Vec<&str> = comp.split('=').collect();
            if let Ok(n) = pair[1].parse::<u32>() {
                // standard fetch with an integer version
                match backend::fetch_and_unpack_component(backend, pair[0], Some(n), Some(env)) {
                    Ok(c) => updated.push(c),
                    Err(e) => {
                        warn!("Failed to update {} ({})", pair[0], e);
                        error = Some(e);
                    }
                }
            } else {
                // fetch from stash - this does not go into `updated` it it succeeds
                // because we wont and cannot save stashed versions in the manifest
                let _ = backend::fetch_from_stash(backend, pair[0], pair[1]).map_err(|e| {
                    warn!("Failed to update {} from stash ({})", pair[0], e);
                    error = Some(e);
                });
            }
        } else {
            // fetch without a specific version (latest)
            match backend::fetch_and_unpack_component(backend, comp, None, Some(env)) {
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
                *hmap.get_mut(&c.name).unwrap() = c.version;
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
pub fn update_all(manifest: &Manifest,
                  backend: &Artifactory,
                  save: bool,
                  dev: bool,
                  env: &str)
                  -> LalResult<()> {
    let deps: Vec<String> = if dev {
        manifest.devDependencies.keys().cloned().collect()
    } else {
        manifest.dependencies.keys().cloned().collect()
    };
    update(manifest, backend, deps, save && !dev, save && dev, env)
}
