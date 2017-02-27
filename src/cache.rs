use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use build::tar_output;
use install;
use super::{CliError, LalResult, Manifest, Config, Lockfile};

pub fn is_cached(cfg: &Config, name: &str, version: u32, env: &str) -> bool {
    get_cache_dir(cfg, name, version, env).is_dir()
}

pub fn get_cache_dir(cfg: &Config, name: &str, version: u32, env: &str) -> PathBuf {
    let leading_pth = if env == "default" {
        // can still happen from `lal export` without -e
        Path::new(&cfg.cache).join("globals")
    } else {
        Path::new(&cfg.cache).join("environments").join(env)
    };
    leading_pth.join(name).join(version.to_string())
}

pub fn store_tarball(cfg: &Config, name: &str, version: u32, env: &str) -> Result<(), CliError> {
    // 1. mkdir -p cfg.cacheDir/$name/$version
    let destdir = get_cache_dir(cfg, name, version, env);
    if !destdir.is_dir() {
        fs::create_dir_all(&destdir)?;
    }
    // 2. stuff $PWD/$name.tar in there
    let tarname = [name, ".tar"].concat();
    let dest = Path::new(&destdir).join(&tarname);
    let src = Path::new(".").join(&tarname);
    if !src.is_file() {
        return Err(CliError::MissingTarball);
    }
    debug!("Move {:?} -> {:?}", src, dest);
    fs::copy(&src, &dest)?;
    fs::remove_file(&src)?;

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

    // convenience edit for lal status here:
    // we edit the lockfile's version key to be "${stashname}"
    // rather than the ugly colony default of "EXPERIMENTAL-${hex}"
    // stashed builds are only used locally so this allows easier inspection
    // full version list is available in `lal ls -f`
    let lf_path = Path::new("OUTPUT").join("lockfile.json");
    let mut lf = Lockfile::from_path(&lf_path, &mf.name)?;
    lf.version = name.to_string();
    lf.write(&lf_path, true)?;

    let destdir = Path::new(&cfg.cache)
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

// helper for install::export
pub fn get_path_to_stashed_component(cfg: &Config,
                                     component: &str,
                                     stashname: &str)
                                     -> LalResult<PathBuf> {
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
    let tarname = get_path_to_stashed_component(cfg, component, stashname)?;
    install::extract_tarball_to_input(tarname, &component)?;
    Ok(())
}

use chrono::{DateTime, UTC, Duration, TimeZone};
use filetime::FileTime;

// helper for `lal::clean`
fn clean_in_dir(cutoff: DateTime<UTC>, dirs: WalkDir) -> LalResult<()> {
    let drs = dirs.into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir());

    for d in drs {
        let pth = d.path();
        trace!("Checking {}", pth.to_str().unwrap());
        let mtime = FileTime::from_last_modification_time(&d.metadata().unwrap());
        let mtimedate = UTC.ymd(1970, 1, 1).and_hms(0, 0, 0) +
                        Duration::seconds(mtime.seconds_relative_to_1970() as i64);

        trace!("Found {} with mtime {}", pth.to_str().unwrap(), mtimedate);
        if mtimedate < cutoff {
            debug!("Cleaning {}", pth.to_str().unwrap());
            fs::remove_dir_all(pth)?;
        }
    }
    Ok(())
}

/// Clean old artifacts in `cfg.cache` cache directory
///
/// This does the equivalent of find CACHEDIR -mindepth 3 -maxdepth 3 -type d
/// With the correct mtime flags, then -exec deletes these folders.
pub fn clean(cfg: &Config, days: i64) -> LalResult<()> {
    let cutoff = UTC::now() - Duration::days(days);
    debug!("Cleaning all artifacts from before {}", cutoff);

    // clean out environment subdirectories
    let edir = Path::new(&cfg.cache).join("environments");
    let edirs = WalkDir::new(&edir).min_depth(3).max_depth(3);
    clean_in_dir(cutoff, edirs)?;

    // clean out stash + globals
    let dirs = WalkDir::new(&cfg.cache).min_depth(3).max_depth(3);
    clean_in_dir(cutoff, dirs)?;

    Ok(())
}
