use std::process::{Command, Stdio};
use std::env;
use std::path::Path;
use std::vec::Vec;

use configure;
use errors::CliError;

pub fn docker_run(cfg: &configure::Config,
                  command: Vec<&str>,
                  interactive: bool)
                  -> Result<(), CliError> {

    let home = env::home_dir().unwrap(); // crash if no $HOME
    let git_cfg = Path::new(&home).join(".gitconfig");
    let pwd = env::current_dir().unwrap();

    let s = try!(Command::new("docker")
                     .arg("run")
                     .arg("-v")
                     .arg(format!("{}:/home/lal/.gitconfig", git_cfg.display()))
                     .arg("-v")
                     .arg(format!("{}:/home/lal/root", pwd.display()))
                     .args(&vec!["-w", "/home/lal/root"])
                     .args(&vec!["--net", "host"])
                     .args(&vec!["--cap-add", "SYS_NICE"])
                     .args(&vec!["--user", "lal"])
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
                     .status());

    if !s.success() {
        return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
    }
    Ok(())
}

pub fn shell(cfg: &configure::Config) -> Result<(), CliError> {
    info!("Entering docker container");
    docker_run(&cfg, vec!["/bin/bash"], true)
}


#[cfg(test)]
mod tests {
    use shell;
    use configure;

    #[test]
    fn can_run_commands() {
        let cfg = configure::current_config().unwrap();
        let r = shell::docker_run(&cfg, vec!["echo", "echo from docker"], false);
        assert_eq!(r.is_ok(), true);
    }
    #[test]
    fn can_touch_mounted_files() {
        let cfg = configure::current_config().unwrap();
        let r = shell::docker_run(&cfg, vec!["touch", "README.md"], false);
        assert_eq!(r.is_ok(), true);
    }

}
