use std::fs;
use std::path::Path;
use std::env;

use configure;
use errors::CliError;

pub fn is_cached(cfg: &configure::Config, name: &str, version: u32) -> bool {
    !Path::new(&cfg.cache).join(&cfg.target).join(name).join(version.to_string()).is_dir()
}

// for the future when we are not fetching from globalroot
pub fn store_tarball(cfg: &configure::Config, name: &str, version: u32) -> Result<(), CliError> {
    // 1. mkdir -p cfg.cacheDir/$target/$name/$version
    let pwd = env::current_dir().unwrap();
    let destdir = Path::new(&cfg.cache).join(&cfg.target).join(name).join(version.to_string());
    if !destdir.is_dir() {
        debug!("where tf is dest? {:?}", destdir);
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
