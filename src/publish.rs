use std::path::Path;
use std::fs::File;

use util::artifactory::upload_tarball;
use super::verify;
use super::{LalResult, CliError, Config, Manifest};

pub fn publish(name: &str, cfg: &Config, mf: &Manifest, env: &str) -> LalResult<()> {
    let artdir = Path::new("./ARTIFACT");
    let tarball = artdir.join(format!("{}.tar.gz", name));
    if !artdir.is_dir() || !tarball.exists() {
        return Err(CliError::MissingReleaseBuild);
    }
    // for safety - verify INPUT
    try!(verify(mf, env));

    // TODO: lockfile sanity checking for where it's going

    // TODO: pass file somehow..
    try!(upload_tarball(&cfg.artifactory));
    Ok(())
}
