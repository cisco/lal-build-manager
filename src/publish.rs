use std::path::Path;
use std::fs::File;

use util::artifactory::upload_tarball;
use super::verify;
use super::{LalResult, CliError, Manifest};

pub fn publish(name: &str, mf: &Manifest, env: &str) -> LalResult<()> {
    let artdir = Path::new("./ARTIFACT");
    let tarball = artdir.join(format!("{}.tar.gz", name));
    if !artdir.is_dir() || !tarball.exists() {
        return Err(CliError::MissingReleaseBuild)
    }
    try!(verify(mf, env));

    // TODO: upload ARTIFACT/{}.tar.gz
    // TODO: run lal verify?
    try!(upload_tarball());
    Ok(())
}
