use std::process::Command;

use configure;
use shell;
use init::Manifest;
use errors::CliError;

pub fn build(cfg: &configure::Config,
             manifest: Manifest,
             name: Option<&str>)
             -> Result<(), CliError> {
    try!(Command::new("mkdir").arg("-p").arg("OUTPUT").output());

    info!("Running build script in docker container");
    let component = name.unwrap_or(&manifest.name);
    // TODO: allow passing in target decorators?
    let cmd = vec!["./BUILD", &component, &cfg.target];
    debug!("Build script is {:?}", cmd);
    shell::docker_run(&cfg, cmd, false)
}
