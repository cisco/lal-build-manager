use std::path::Path;
use std::fs::File;

use util::artifactory::upload_artifact;
use super::{LalResult, CliError, Config, Lockfile};

pub fn publish(name: &str, cfg: &Config, env: &str) -> LalResult<()> {
    let artdir = Path::new("./ARTIFACT");
    let tarball = artdir.join(format!("{}.tar.gz", name));
    let lockfile = artdir.join("lockfile.json");
    if !artdir.is_dir() || !tarball.exists() {
        return Err(CliError::MissingReleaseBuild);
    }

    let lock = try!(Lockfile::release_build());

    let version = try!(lock.version.parse::<u32>().map_err(|e| {
        error!("Release build not done --with-version=$BUILD_VERSION");
        debug!("Error: {}", e);
        CliError::MissingReleaseBuild
    }));

    let build_env = try!(lock.environment.ok_or_else(|| {
        error!("Release build has no environment");
        CliError::MissingReleaseBuild
    }));
    assert_eq!(env, build_env); // for now


    info!("Publishing {}={}", name, version);

    let tar_uri = format!("{}/{}/{}.tar.gz", name, version, name);
    let mut tarf = try!(File::open(tarball));
    try!(upload_artifact(&cfg.artifactory, tar_uri, &mut tarf));

    let mut lockf = try!(File::open(lockfile));
    let lf_uri = format!("{}/{}/lockfile.json", name, version);
    try!(upload_artifact(&cfg.artifactory, lf_uri, &mut lockf));

    Ok(())
}
