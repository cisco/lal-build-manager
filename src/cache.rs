use std::fs;
use std::path::Path;
use std::env;

use configure::Config;
use init::Manifest;
use errors::{CliError, LalResult};

pub fn is_cached(cfg: &Config, name: &str, version: u32) -> bool {
    !Path::new(&cfg.cache)
        .join(name)
        .join(version.to_string())
        .is_dir()
}

// for the future when we are not fetching from globalroot
pub fn store_tarball(cfg: &Config, name: &str, version: u32) -> Result<(), CliError> {
    // 1. mkdir -p cfg.cacheDir/$name/$version
    let pwd = env::current_dir().unwrap();
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
    let src = Path::new(&pwd).join(&tarname);
    if !src.is_file() {
        return Err(CliError::MissingTarball);
    }
    debug!("Move {:?} -> {:?}", src, dest);
    try!(fs::copy(&src, &dest));
    try!(fs::remove_file(&src));

    // 3. TODO: get metadata as well in there?

    // Done
    Ok(())
}

pub fn stash(cfg: Config, mf: Manifest, name: &str) -> LalResult<()> {
    info!("Stashing OUTPUT into cache under {}/{}", mf.name, name);

    let pwd = env::current_dir().unwrap();
    let outputdir = Path::new(&pwd).join("OUTPUT");
    if !outputdir.is_dir() {
        return Err(CliError::MissingBuild);
        // TODO: need to verify lockfile here
    }
    let destdir = Path::new(&cfg.cache)
        .join("stash")
        .join(mf.name)
        .join(name);
    debug!("Creating {:?}", destdir);
    try!(fs::create_dir_all(&destdir));

    // Need to implement build before doing the rest here
    unimplemented!();
}
