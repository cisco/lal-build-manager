use std::fs;
use std::path::Path;

use backend::{Artifactory, tar_output};
use super::{CliError, LalResult, Manifest, Lockfile};


/// Saves current build `./OUTPUT` to the local cache under a specific name
///
/// This tars up `/OUTPUT` similar to how `build` is generating a tarball,
/// then copies this to `~/.lal/cache/stash/${name}/`.
///
/// This file can then be installed via `update` using a component=${name} argument.
pub fn stash(backend: &Artifactory, mf: &Manifest, name: &str) -> LalResult<()> {
    info!("Stashing OUTPUT into cache under {}/{}", mf.name, name);
    // sanity: verify name does NOT parse as a u32
    if let Ok(n) = name.parse::<u32>() {
        return Err(CliError::InvalidStashName(n));
    }

    let outputdir = Path::new("./OUTPUT");
    if !outputdir.is_dir() {
        return Err(CliError::MissingBuild);
    }

    // convenience edit for lal status here:
    // we edit the lockfile's version key to be "${stashname}"
    // rather than the ugly colony default of "EXPERIMENTAL-${hex}"
    // stashed builds are only used locally so this allows easier inspection
    // full version list is available in `lal ls -f`
    let lf_path = Path::new("OUTPUT").join("lockfile.json");
    let mut lf = Lockfile::from_path(&lf_path, &mf.name)?;
    lf.version = name.to_string();
    lf.write(&lf_path, true)?;

    let destdir = Path::new(&backend.cache)
        .join("stash")
        .join(&mf.name)
        .join(name);
    debug!("Creating {:?}", destdir);
    fs::create_dir_all(&destdir)?;

    // Tar it straight into destination
    tar_output(&destdir.join(format!("{}.tar.gz", mf.name)))?;

    // Copy the lockfile there for sanity
    // NB: this is not really needed, as it's included in the tarball anyway
    fs::copy("./OUTPUT/lockfile.json", destdir.join("lockfile.json"))?;

    Ok(())
}
