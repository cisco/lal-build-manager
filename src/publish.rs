use std::path::Path;
use std::fs::File;

use util::artifactory::upload_artifact;
use super::{LalResult, CliError, Config, Lockfile};

pub fn publish(name: &str, cfg: &Config, env: &str) -> LalResult<()> {
    let artdir = Path::new("./ARTIFACT");
    let tarball = artdir.join(format!("{}.tar.gz", name));
    if !artdir.is_dir() || !tarball.exists() {
        return Err(CliError::MissingReleaseBuild);
    }

    let lf = try!(Lockfile::release_build());

    let version = try!(lf.version.parse::<u32>().map_err(|e| {
        error!("Release build not done --with-version=$BUILD_VERSION");
        debug!("Error: {}", e);
        CliError::MissingReleaseBuild
    }));

    let build_env = try!(lf.environment.ok_or_else(|| {
        error!("Release build has no environment");
        CliError::MissingReleaseBuild
    }));
    assert_eq!(env, build_env); // for now

    let uri = format!("{}/{}/{}.tar.gz", name, version, name);
    let mut f = try!(File::open(tarball));
    info!("PUT {}", uri);
    try!(upload_artifact(&cfg.artifactory, uri, &mut f));

    Ok(())
}
