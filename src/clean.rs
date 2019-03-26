use std::fs;
use std::path::Path;

use chrono::{DateTime, Duration, TimeZone, UTC};
use filetime::FileTime;
use walkdir::WalkDir;

use super::LalResult;

// helper for `lal::clean`
fn clean_in_dir(cutoff: DateTime<UTC>, dirs: WalkDir) -> LalResult<()> {
    let dirs = dirs
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir());

    for d in dirs {
        let pth = d.path();
        trace!("Checking {}", pth.to_str().unwrap());
        let mtime = FileTime::from_last_modification_time(&d.metadata().unwrap());
        let mtimedate = UTC.ymd(1970, 1, 1).and_hms(0, 0, 0)
            + Duration::seconds(mtime.seconds_relative_to_1970() as i64);

        trace!("Found {} with mtime {}", pth.to_str().unwrap(), mtimedate);
        if mtimedate < cutoff {
            debug!("Cleaning {}", pth.to_str().unwrap());
            fs::remove_dir_all(pth)?;
        }
    }
    Ok(())
}

/// Clean old artifacts in cache directory
///
/// This does the equivalent of find CACHEDIR -mindepth 3 -maxdepth 3 -type d
/// With the correct mtime flags, then -exec deletes these folders.
pub fn clean(cachedir: &str, days: i64) -> LalResult<()> {
    let cutoff = UTC::now() - Duration::days(days);
    debug!("Cleaning all artifacts from before {}", cutoff);

    // clean out environment subdirectories
    let dir = Path::new(&cachedir).join("environments");
    let dirs = WalkDir::new(&dir).min_depth(3).max_depth(3);
    clean_in_dir(cutoff, dirs)?;

    // clean out stash
    let dirs = WalkDir::new(&cachedir).min_depth(3).max_depth(3);
    clean_in_dir(cutoff, dirs)?;

    Ok(())
}
