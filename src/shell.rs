use std::process::{Command, Stdio};
use std::env;
use std::path::Path;
use std::vec::Vec;

use configure;
use errors::{CliError, LalResult};


/// Runs an arbitrary command in the configured docker environment
///
/// This will mount the current directory as `~/volume` as well as a few conveniences,
/// and absorb the `Stdio` supplied by this `Command`.
///
/// This is the most general function, used by both `lal build` and `lal shell`.
pub fn docker_run(cfg: &configure::Config,
                  command: Vec<String>,
                  interactive: bool)
                  -> LalResult<()> {
    trace!("Finding home and cwd");
    let home = env::home_dir().unwrap(); // crash if no $HOME
    let git_cfg = Path::new(&home).join(".gitconfig");
    let pwd = env::current_dir().unwrap();

    trace!("docker run");
    trace!(" - mounting {}", git_cfg.display());
    trace!(" - mounting {}", pwd.display());
    let s = try!(Command::new("docker")
        .arg("run")
        .arg("-v")
        .arg(format!("{}:/home/lal/.gitconfig", git_cfg.display()))
        .arg("-v")
        .arg(format!("{}:/home/lal/volume", pwd.display()))
        .args(&vec!["-w", "/home/lal/volume"])
        .args(&vec!["--net", "host"])
        .args(&vec!["--cap-add", "SYS_NICE"])
        .args(&vec!["--user", "lal"])
        .arg(if interactive { "-it" } else { "-t" })
        .arg(&cfg.container)
        .args(&command)
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status());

    trace!("Exited docker");
    if !s.success() {
        return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
    }
    Ok(())
}

/// Mounts and enters `.` in an interactive bash shell using the configured container.
pub fn shell(cfg: &configure::Config) -> LalResult<()> {
    info!("Entering docker container");
    docker_run(&cfg, vec!["/bin/bash".to_string()], true)
}
