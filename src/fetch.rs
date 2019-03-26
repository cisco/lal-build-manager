use fs2::FileExt;
use std::fs;
use std::path::Path;

use super::{CliError, Coordinates, LalResult, Lockfile, Manifest};
use crate::channel::Channel;
use crate::storage::{Backend, CachedBackend};
use crate::verify;

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
pub fn fetch<T: CachedBackend + Backend + ?Sized>(
    manifest: &Manifest,
    backend: &T,
    core: bool,
    env: &str,
) -> LalResult<()> {
    // first ensure manifest is sane:
    // do not check whether testing components are allowed now
    manifest.verify(verify::Flags::TESTING)?;

    debug!(
        "Installing dependencies{}",
        if core { "" } else { " and devDependencies" }
    );

    // create the joined hashmap of dependencies and possibly devdependencies
    let mut deps = if core {
        manifest.dependencies.clone()
    } else {
        manifest.all_dependencies()
    };
    let mut extraneous = vec![]; // stuff we should remove

    // figure out what we have already
    let lf = Lockfile::default().populate_from_input().map_err(|e| {
        // Guide users a bit if they did something dumb - see #77
        warn!("Populating INPUT data failed - your INPUT may be corrupt");
        warn!("This can happen if you CTRL-C during `lal fetch`");
        warn!("Try to `rm -rf INPUT` and `lal fetch` again.");
        e
    })?;
    // filter out what we already have (being careful to examine env)
    for (name, d) in lf.dependencies {
        // if d.name at d.version in d.environment matches something in deps
        if let Some(coords) = deps.get(&name) {
            let (ver, channel) = match coords {
                Coordinates::OneD(v) => (*v, None),
                Coordinates::TwoD(c) => (c.version, Some(&c.channel)),
            };
            // Parse channel and compare.
            let channel = Channel::from_option(&channel);
            // version found in manifest
            // ignore non-integer versions (stashed things must be overwritten)
            if let Ok(n) = d.version.parse::<u32>() {
                if n == ver && d.environment == env && Channel::from_option(&d.channel) == channel {
                    info!("Reuse {} {} {}", env, name, n);
                    deps.remove(&name);
                }
            }
        } else {
            extraneous.push(name.clone());
        }
    }

    // Fetch time -- acquire a file lock (not a lockfile!) in case other lal instances are running
    // This was a bug with multiple executors, easy to reproduce by running two instances of
    // `lal fetch` at the same time on a single machine
    let mut err = None;
    if !deps.is_empty() {
        let cache_dir = backend.get_cache_dir();
        let cache_path = Path::new(&cache_dir);
        if !cache_path.is_dir() {
            fs::create_dir_all(&cache_path)?;
        }

        debug!("Acquiring backend lock...");
        let file_lock_path = Path::new(&cache_dir).join("fetch.lock");
        let file_lock = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(file_lock_path)?;
        file_lock.lock_exclusive()?; // block until this process can lock the file
        debug!("Lock acquired");

        for (k, c) in deps {
            info!("Fetch {} {}={}", env, k, c);

            let (version, channel) = match c {
                Coordinates::OneD(v) => (v, None),
                Coordinates::TwoD(c) => (c.version, Some(c.channel)),
            };

            let channel = Channel::from_option(&channel);

            // first kill the folders we actually need to fetch:
            let cmponent_dir = Path::new("./INPUT").join(&k);
            if cmponent_dir.is_dir() {
                // Don't think this can fail, but we are dealing with NFS
                fs::remove_dir_all(&cmponent_dir).map_err(|e| {
                    warn!("Failed to remove INPUT/{} - {}", k, e);
                    warn!("Please clean out your INPUT folder yourself to avoid corruption");
                    e
                })?;
            }

            let _ = backend
                .unpack_published_component(&k, Some(version), env, &channel)
                .map_err(|e| {
                    warn!("Failed to completely install {} ({})", k, e);
                    // likely symlinks inside tarball that are being dodgy
                    // this is why we clean_input
                    err = Some(e);
                });
        }

        debug!("Releasing backend lock..");
        file_lock.unlock()?;
        debug!("Lock released");
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
