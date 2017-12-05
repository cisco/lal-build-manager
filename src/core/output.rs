use std::process::Command;
use std::path::Path;

use super::{CliError, LalResult};

/// Helper for stash and build
pub fn tar(tarball: &Path) -> LalResult<()> {
    info!("Taring OUTPUT");
    let mut args: Vec<String> = vec![
        "czf".into(),
        tarball.to_str().unwrap().into(), // path created internally - always valid unicode
        "--transform=s,^OUTPUT/,,".into(), // remove leading OUTPUT
    ];

    // Avoid depending on wildcards (which would also hide hidden files)
    // All links, hidden files, and regular files should go into the tarball.
    let findargs = vec!["OUTPUT/", "-type", "f", "-o", "-type", "l"];
    debug!("find {}", findargs.join(" "));
    let find_output = Command::new("find").args(&findargs).output()?;
    let find_str = String::from_utf8_lossy(&find_output.stdout);

    // append each file as an arg to the main tar process
    for f in find_str.trim().split('\n') {
        args.push(f.into())
    }

    // basically `tar czf component.tar.gz --transform.. $(find OUTPUT -type f -o -type l)`:
    debug!("tar {}", args.join(" "));
    let s = Command::new("tar").args(&args).status()?;

    if !s.success() {
        return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
    }
    Ok(())
}
