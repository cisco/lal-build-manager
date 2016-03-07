use std::fs;
use std::path::Path;
use std::env;

use configure;
use errors::CliError;

pub fn is_cached(cfg: &configure::Config, name: &str, version: u32) -> bool {
    let pwd = env::current_dir().unwrap();
    let destdir = Path::new(&pwd).join(&cfg.target).join(name).join(version.to_string());
    !destdir.is_dir()
}

// for the future when we are not fetching from globalroot
pub fn cache_tarball(cfg: &configure::Config, name: &str, version: u32) -> Result<(), CliError> {
    // 1. mkdir -p cfg.cacheDir/$target/$name/$version
    let pwd = env::current_dir().unwrap();
    let destdir = Path::new(&pwd).join(&cfg.target).join(name).join(version.to_string());
    if !destdir.is_dir() {
        try!(fs::create_dir_all(&destdir));
    }
    // 2. stuff $PWD/$name.tar in there
    let dest = Path::new(&destdir).join(&name).join(".tar");
    let src = Path::new(&pwd).join(&name).join(".tar");
    try!(fs::rename(src, dest));

    // 3. TODO: get metadata as well in there?

    // Done
    Ok(())
}
