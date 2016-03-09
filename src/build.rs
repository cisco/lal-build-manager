use std::process::Command;

use configure::Config;
use shell;
use init::Manifest;
use errors::LalResult;

pub fn build(cfg: &Config, manifest: &Manifest, name: Option<&str>) -> LalResult<()> {
    try!(Command::new("mkdir").arg("-p").arg("OUTPUT").output());

    info!("Running build script in docker container");
    let component = name.unwrap_or(&manifest.name);
    // TODO: allow passing in target decorators?
    let cmd = vec!["./BUILD", &component, &cfg.target];
    debug!("Build script is {:?}", cmd);
    try!(shell::docker_run(&cfg, cmd, false));
    Ok(())
}
