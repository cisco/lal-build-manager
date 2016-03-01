use configure;
use shell;
use init;

pub fn build(cfg: &configure::Config) {
    println!("Running build script in docker container");
    let manifest = init::read_manifest().unwrap();
    let cmd = vec!["./BUILD", &manifest.name, &cfg.target];
    println!("Build script is {:?}", cmd);
    shell::docker_run(&cfg, cmd, false);
}
