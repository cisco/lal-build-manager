use std::fs;
use std::path::Path;

use configure::Config;
use init::Manifest;
use errors::{CliError, LalResult};

pub fn is_cached(cfg: &Config, name: &str, version: u32) -> bool {
    !Path::new(&cfg.cache)
        .join(name)
        .join(version.to_string())
        .is_dir()
}

pub fn store_tarball(cfg: &Config, name: &str, version: u32) -> Result<(), CliError> {
    // 1. mkdir -p cfg.cacheDir/$name/$version
    let destdir = Path::new(&cfg.cache)
        .join("globals")
        .join(name)
        .join(version.to_string());
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
/// This file can then be installed via `install` using a component=${name} argument.
/// TODO: complete this
pub fn stash(cfg: Config, mf: Manifest, name: &str) -> LalResult<()> {
    info!("Stashing OUTPUT into cache under {}/{}", mf.name, name);
    // TODO: verify name does NOT parse as a u32

    let outputdir = Path::new("./OUTPUT");
    if !outputdir.is_dir() {
        return Err(CliError::MissingBuild);
    }
    let destdir = Path::new(&cfg.cache)
        .join("stash")
        .join(mf.name)
        .join(name);
    debug!("Creating {:?}", destdir);
    try!(fs::create_dir_all(&destdir));

    // Need to implement build before doing the rest here
    unimplemented!();
    // TODO: then implement install from stash
    // could actually implement `lal reuse ciscossl` and not use install for this..
}
