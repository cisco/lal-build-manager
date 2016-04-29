use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
pub fn stash(cfg: &Config, mf: &Manifest, name: &str) -> LalResult<()> {
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

// helper for install::export
pub fn get_path_to_stashed_component(cfg: &Config, component: &str, stashname: &str) -> LalResult<PathBuf> {
    let stashdir = Path::new(&cfg.cache).join("stash").join(component).join(stashname);
    if !stashdir.is_dir() {
        return Err(CliError::MissingStashArtifact(format!("{}/{}", component, stashname)));
    }
    debug!("Inferring stashed version {} of component {}",
           stashname,
           component);
    let tarname = stashdir.join(format!("{}.tar.gz", component));
    Ok(tarname)
}

// helper for install::update
pub fn fetch_from_stash(cfg: &Config, component: &str, stashname: &str) -> LalResult<()> {
    let tarname = try!(get_path_to_stashed_component(cfg, component, stashname));
    try!(install::extract_tarball_to_input(tarname, &component));
    Ok(())
}

/// Clean old artifacts in `cfg.cache` cache directory
///
/// This does the equivalent of find CACHEDIR -mindepth 3 -maxdepth 3 -type d
/// With the correct mtime flags, then -exec deletes these folders.
pub fn clean(cfg: &Config, days: i64) -> LalResult<()> {
    use filetime::FileTime;
    use chrono::*;

    let dirs = WalkDir::new(&cfg.cache)
        .min_depth(3)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir());

    let cutoff = UTC::now() - Duration::days(days);
    debug!("Cleaning all artifacts from before {}", cutoff);
    for d in dirs {
        let pth = d.path();
        trace!("Checking {}", pth.to_str().unwrap());
        let mtime = FileTime::from_last_modification_time(&d.metadata().unwrap());
        let mtimedate = UTC.ymd(1970, 1, 1).and_hms(0, 0, 0) +
                        Duration::seconds(mtime.seconds_relative_to_1970() as i64);

        trace!("Found {} with mtime {}", pth.to_str().unwrap(), mtimedate);
        if mtimedate < cutoff {
            debug!("Cleaning {}", pth.to_str().unwrap());
            try!(fs::remove_dir_all(pth));
        }
    }
    Ok(())
}
