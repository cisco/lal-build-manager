use std::process::{Command, Stdio};
use std::env;
use std::path::Path;

use configure;

pub fn docker_run(cfg: &configure::Config, command: &str, interactive: bool) {

    let home = env::home_dir().unwrap(); // crash if no $HOME
    let git_cfg = Path::new(&home).join(".gitconfig");
    let pwd = env::current_dir().unwrap();

    // TODO: command does not work if it contains whitespaces, .args?
    let output = Command::new("docker")
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
                     .arg(if interactive { "-it" } else { "-t" })
                     .arg("edonusdevelopers/centos_build")
                     .arg(&command)
                     .stdout(Stdio::inherit())
                     .stdin(Stdio::inherit())
                     .stderr(Stdio::inherit())
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
}

pub fn shell(cfg: &configure::Config) {
    println!("Entering docker container");
    docker_run(&cfg, "/bin/bash", true);
}
