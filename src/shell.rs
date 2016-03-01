use std::process::{Command, Stdio};
use std::env;
use std::path::Path;
use std::vec::Vec;

use configure;

pub fn docker_run(cfg: &configure::Config, command: Vec<&str>, interactive: bool) {

    let home = env::home_dir().unwrap(); // crash if no $HOME
    let git_cfg = Path::new(&home).join(".gitconfig");
    let pwd = env::current_dir().unwrap();

    Command::new("docker")
        .arg("run")
        .arg("-v")
        .arg(format!("{}:/home/lal/.gitconfig", git_cfg.display()))
        .arg("-v")
        .arg(format!("{}:/home/lal/root", pwd.display()))
        .arg("-w")
        .arg("/home/lal/root")
        .arg("--net")
        .arg("host")
        .arg("--cap-add")
        .arg("SYS_NICE")
        .arg("--user")
        .arg("lal")
        .arg(if interactive {
            "-it"
        } else {
            "-t"
        })
        .arg(&cfg.container)
        .args(&command)
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
}

pub fn shell(cfg: &configure::Config) {
    println!("Entering docker container");
    docker_run(&cfg, vec!["/bin/bash"], true);
}
