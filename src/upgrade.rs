//! This file controls the automatic upgrade procedure in lal for musl builds.
//!
//! It will, if a new version is available in the `Backend`, download it
//! and overwrite the running executable using a file renaming dance.
//!
//! Be very careful about updating these functions without also testing the musl
//! build on a variety of Cargo.toml.version strings.
//!
//! People should not have to be told to `curl lal.tar.gz | tar xz -C prefix` again.

use semver::Version;
use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;

use super::{LalResult, CliError};
use super::{http_download_to_path, get_latest_lal_version, LatestLal};

struct ExeInfo {
    /// Whether ldd things its a dynamic executable
    dynamic: bool,
    /// Whether this is a debug build (only for dynamic executables)
    debug: bool,
    /// Path to current_exe
    path: String,
    /// Best guess at install prefix based on path (only for static executables)
    prefix: Option<PathBuf>,
    /// Parsed semver version
    version: Version,
}

fn identify_exe() -> LalResult<ExeInfo> {
    let pth = env::current_exe()?;
    trace!("lal at {}", pth.display());
    let ldd_output = Command::new("ldd").arg(&pth).output()?;
    let ldd_str = String::from_utf8_lossy(&ldd_output.stdout);
    let is_dynamic = !ldd_str.contains("not a dynamic executable");
    let pthstr: String = pth.to_str().unwrap().into();
    let prefix = if pthstr.contains("/bin/") {
        let v: Vec<&str> = pthstr.split("/bin/").collect();
        if v.len() == 2 { Some(Path::new(v[0]).to_owned()) } else { None }
    } else {
        None
    };
    Ok(ExeInfo {
           dynamic: is_dynamic,
           debug: pthstr.contains("debug"), // cheap check for compiled versions
           path: pthstr,
           prefix: prefix,
           version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
       })
}

// basic tarball extractor
// smaller than the INPUT extractor uses because it doesn't clear out anything
fn extract_tarball(input: PathBuf, output: &PathBuf) -> LalResult<()> {
    use tar::Archive;
    use flate2::read::GzDecoder;

    let data = fs::File::open(input)?;
    let decompressed = GzDecoder::new(data)?; // decoder reads data
    let mut archive = Archive::new(decompressed); // Archive reads decoded

    archive.unpack(&output)?;
    Ok(())
}

fn verify_permissions(exe: &ExeInfo) -> LalResult<()> {
    // this is sufficient unless the user copied it over manually with sudo
    // and then chowned it, but for all normal installs, touching the main file
    // would sufficiently check that we have write permissions
    let s = Command::new("touch").arg(&exe.path).status()?;
    if !s.success() {
        return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
    }
    Ok(())
}

fn overwrite_exe(latest: &LatestLal, exe: &ExeInfo) -> LalResult<()> {
    let prefix = exe.prefix.clone().unwrap();
    extract_tarball(prefix.join("lal.tar.gz"), &prefix)?;
    validate_exe(latest, exe)?;
    Ok(())
}

fn validate_exe(latest: &LatestLal, exe: &ExeInfo) -> LalResult<()> {
    let lal_output = Command::new(&exe.path).arg("-V").output()?;
    let lal_str = String::from_utf8_lossy(&lal_output.stdout);
    debug!("Output from lal -V: {}", lal_str.trim());
    debug!("Expecting to find: {}", latest.version.to_string());
    if !lal_str.contains(&latest.version.to_string()) {
        let estr = format!("lal -V yielded {}", lal_str.trim());
        return Err(CliError::UpgradeValidationFailure(estr));
    }
    debug!("New version validated");
    Ok(())
}

fn upgrade_exe(latest: &LatestLal, exe: &ExeInfo) -> LalResult<()> {
    let prefix = exe.prefix.clone().unwrap();
    // 0. sanity - could we actually upgrade if we tried?
    verify_permissions(exe)
        .map_err(|_| CliError::MissingPrefixPermissions(prefix.to_string_lossy().into()))?;
    debug!("Have permissions to write in {}", prefix.display());

    // 1. rename current running executable to the same except _old suffix
    let old_file = prefix.join("bin").join("_lal_old");
    if old_file.is_file() {
        // remove previous upgrade backup
        fs::remove_file(&old_file)?;
    }
    // 2. make sure we can download the tarball before starting
    let tar_dest = prefix.join("lal.tar.gz");
    info!("Downloading tarball to {}", tar_dest.display());
    http_download_to_path(&latest.url, &tar_dest)?;
    info!("Backing up {} to {}", exe.path, old_file.display());
    fs::rename(&exe.path, &old_file)?; // need to undo this if we fail
    // NB: DO NOT INSERT CALLS THAT CAN FAIL HERE BEFORE THE OVERWRITE
    // 3. force dump lal tarball into exe.prefix - rollback if it failed
    info!("Unpacking new version of lal into {}", prefix.display());
    match overwrite_exe(latest, exe) {
        // NB: This call takes a small amount of time - and can be aborted :/
        // it is not an atomic operation, so recovery can unfortunately fail :|
        Ok(_) => trace!("overwrite successful"),
        Err(e) => {
            // tarball could potentially fail to extract here
            warn!("lal upgrade failed - rolling back");
            warn!("Error: {}", e);
            fs::rename(&old_file, &exe.path)?; // better hope this works..
            return Err(e);
        }
    }

    Ok(()) // we did it!
}


/// Check for and possibly upgrade lal when using musl releases
///
/// This will query for the latest version, and upgrade in the one possible case.
/// If a newer version found (> in semver), and it's a static executable,
/// then an executable upgrade is attempted from the new release url.
pub fn upgrade(silent: bool) -> LalResult<bool> {
    let latest = get_latest_lal_version()?;
    let exe = identify_exe()?;

    if latest.version > exe.version {
        // New version found - always full output now
        info!("A new version of lal is available: {}", latest.version);
        info!("You are running {} at {}", exe.version, exe.path);
        println!("");

        if exe.dynamic {
            info!("Your version is built from source - please run (in source checkout):");
            let build_flag = if exe.debug { "" } else { "--release" };
            info!("rustup update stable && git pull && cargo build {}",
                  build_flag);
        } else if exe.prefix.is_some() {
            // install lal in the prefix it's normally in
            info!("Upgrading...");
            upgrade_exe(&latest, &exe)?;
            info!("lal upgraded successfully to {} at {}",
                  latest.version,
                  exe.path);
            println!("");
        } else {
            // static, but no good guess of where to install - let user decide:
            info!("Your version is prebuilt but installed weirdly - please run:");
            info!("curl {} | tar xz -C /usr/local", latest.url);
        }
    } else if silent {
        debug!("You are running the latest version of lal");
    } else {
        info!("You are running the latest version of lal");
    }
    Ok(latest.version > exe.version)
}
