use std::fs;
use std::path::{Path, PathBuf};

use configure::Config;
use init::Manifest;
use build::tar_output;
use install;
use errors::{CliError, LalResult};

pub fn is_cached(cfg: &Config, name: &str, version: u32) -> bool {
    get_cache_dir(cfg, name, version).is_dir()
}

pub fn get_cache_dir(cfg: &Config, name: &str, version: u32) -> PathBuf {
    Path::new(&cfg.cache)
        .join("globals")
        .join(name)
        .join(version.to_string())
}

pub fn store_tarball(cfg: &Config, name: &str, version: u32) -> Result<(), CliError> {
    // 1. mkdir -p cfg.cacheDir/$name/$version
    let destdir = get_cache_dir(cfg, name, version);
    if !destdir.is_dir() {
        try!(fs::create_dir_all(&destdir));
    }
    // 2. stuff $PWD/$name.tar in there
    let tarname = [name, ".tar"].concat();
    let dest = Path::new(&destdir).join(&tarname);
    let src = Path::new(".").join(&tarname);
    if !src.is_file() {
        return Err(CliError::MissingTarball);
    }
    debug!("Move {:?} -> {:?}", src, dest);
    try!(fs::copy(&src, &dest));
    try!(fs::remove_file(&src));

    // NB: in the lockfile is in the tarball - okay for now

    // Done
    Ok(())
}

/// Saves current build `./OUTPUT` to the local cache under a specific name
///
/// This tars up `/OUTPUT` similar to how `build` is generating a tarball,
/// then copies this to `~/.lal/cache/stash/${name}/`.
///
/// This file can then be installed via `update` using a component=${name} argument.
pub fn stash(cfg: Config, mf: Manifest, name: &str) -> LalResult<()> {
    info!("Stashing OUTPUT into cache under {}/{}", mf.name, name);
    // sanity: verify name does NOT parse as a u32
    if let Ok(n) = name.parse::<u32>() {
        return Err(CliError::InvalidStashName(n));
    }

    let outputdir = Path::new("./OUTPUT");
    if !outputdir.is_dir() {
        return Err(CliError::MissingBuild);
    }
    let destdir = Path::new(&cfg.cache)
        .join("stash")
        .join(&mf.name)
        .join(name);
    debug!("Creating {:?}", destdir);
    try!(fs::create_dir_all(&destdir));

    // Tar it straight into destination
    try!(tar_output(&destdir.join(format!("{}.tar.gz", mf.name))));

    // Copy the lockfile there for sanity
    // NB: this is not really needed, as it's included in the tarball anyway
    try!(fs::copy("./OUTPUT/lockfile.json", destdir.join("lockfile.json")));

    Ok(())
}

// helper for install::update
pub fn fetch_from_stash(cfg: &Config, component: &str, stashname: &str) -> LalResult<()> {
    let stashdir = Path::new(&cfg.cache).join("stash").join(component).join(stashname);
    if !stashdir.is_dir() {
        return Err(CliError::MissingStashArtifact(format!("{}/{}", component, stashname)));
    }
    // grab it and dump it into INPUT
    debug!("Fetching stashed version {} of component {}", stashname, component);
    let tarname = stashdir.join(format!("{}.tar.gz", component));
    try!(install::extract_tarball_to_input(tarname, &component));
    Ok(())
}
