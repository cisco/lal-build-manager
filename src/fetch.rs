use std::fs;
use std::path::Path;

use storage::CachedBackend;
use super::{CliError, LalResult, Lockfile, Manifest};

fn clean_input() {
    let input = Path::new("./INPUT");
    if input.is_dir() {
        fs::remove_dir_all(&input).unwrap();
    }
}

/// Fetch all dependencies from `manifest.json`
///
/// This will read, and HTTP GET all the dependencies at the specified versions.
/// If the `core` bool is set, then `devDependencies` are not installed.
pub fn fetch<T: CachedBackend + ?Sized>(
    manifest: &Manifest,
    backend: &T,
    core: bool,
    env: &str,
) -> LalResult<()> {
    // first ensure manifest is sane:
    manifest.verify()?;

    debug!("Installing dependencies{}",
           if !core { " and devDependencies" } else { "" });

    // create the joined hashmap of dependencies and possibly devdependencies
    let mut deps = manifest.dependencies.clone();
    if !core {
        for (k, v) in &manifest.devDependencies {
            deps.insert(k.clone(), *v);
        }
    }
    let mut extraneous = vec![]; // stuff we should remove

    // figure out what we have already
    let lf = Lockfile::default()
        .populate_from_input()
        .map_err(|e| {
            // Guide users a bit if they did something dumb - see #77
            warn!("Populating INPUT data failed - your INPUT may be corrupt");
            warn!("This can happen if you CTRL-C during `lal fetch`");
            warn!("Try to `rm -rf INPUT` and `lal fetch` again.");
            e
        })?;
    // filter out what we already have (being careful to examine env)
    for (name, d) in lf.dependencies {
        // if d.name at d.version in d.environment matches something in deps
        if let Some(&cand) = deps.get(&name) {
            // version found in manifest
            // ignore non-integer versions (stashed things must be overwritten)
            if let Ok(n) = d.version.parse::<u32>() {
                if n == cand && d.environment == env {
                    info!("Reuse {} {} {}", env, name, n);
                    deps.remove(&name);
                }
            }
        } else {
            extraneous.push(name.clone());
        }
    }

    let mut err = None;
    for (k, v) in deps {
        info!("Fetch {} {} {}", env, k, v);

        // first kill the folders we actually need to fetch:
        let cmponent_dir = Path::new("./INPUT").join(&k);
        if cmponent_dir.is_dir() {
            // Don't think this can fail, but we are dealing with NFS
            fs::remove_dir_all(&cmponent_dir)
                .map_err(|e| {
                    warn!("Failed to remove INPUT/{} - {}", k, e);
                    warn!("Please clean out your INPUT folder yourself to avoid corruption");
                    e
                })?;
        }

        let _ = backend.unpack_published_component(&k, Some(v), env).map_err(|e| {
            warn!("Failed to completely install {} ({})", k, e);
            // likely symlinks inside tarball that are being dodgy
            // this is why we clean_input
            err = Some(e);
        });
    }

    // remove extraneous deps
    for name in extraneous {
        info!("Remove {}", name);
        let pth = Path::new("./INPUT").join(&name);
        if pth.is_dir() {
            fs::remove_dir_all(&pth)?;
        }
    }

    if err.is_some() {
        warn!("Cleaning potentially broken INPUT");
        clean_input(); // don't want to risk having users in corrupted states
        return Err(CliError::InstallFailure);
    }
    Ok(())
}
