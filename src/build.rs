use configure;
use shell;
use init;
use std::process::Command;

pub fn build(cfg: &configure::Config) {
    // Create OUTPUT
    Command::new("mkdir").arg("-p").arg("OUTPUT").output().unwrap_or_else(|e| {
        panic!("failed to create OUTPUT dir {}", e);
    });

    info!("Running build script in docker container");
    let manifest = init::read_manifest().unwrap();
    let cmd = vec!["./BUILD", &manifest.name, &cfg.target];
    debug!("Build script is {:?}", cmd);
    shell::docker_run(&cfg, cmd, false);
}
