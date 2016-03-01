use configure;
use shell;

pub fn build(cfg: &configure::Config) {
    println!("Running build script in docker container");
    // TODO: needs manifest as well
    shell::docker_run(&cfg, "ls", false);
}
