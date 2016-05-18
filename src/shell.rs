use std::process::Command;
use std::env;
use std::path::Path;
use std::vec::Vec;

use {Config, CliError, LalResult};


/// Runs an arbitrary command in the configured docker environment
///
/// This will mount the current directory as `~/volume` as well as a few conveniences,
/// and absorb the `Stdio` supplied by this `Command`.
///
/// This is the most general function, used by both `lal build` and `lal shell`.
pub fn docker_run(cfg: &Config,
                  command: Vec<String>,
                  interactive: bool,
                  printonly: bool)
                  -> LalResult<()> {
    trace!("Finding home and cwd");
    let home = env::home_dir().unwrap(); // crash if no $HOME
    let git_cfg = Path::new(&home).join(".gitconfig");
    let pwd = env::current_dir().unwrap();

    trace!("docker run");

    let mut mounts : Vec<String> = vec![];
    for mount in cfg.mounts.clone() {
        trace!(" - mounting {}", mount.src);
        mounts.push("-v".to_string());
        let mnt = format!("{}:{}{}", mount.src, mount.dest, if mount.readonly { ":ro" } else { "" });
        mounts.push(mnt);
    }
    mounts.push("-v".to_string());
    mounts.push(format!("{}:/home/lal/.gitconfig:ro", git_cfg.display()));
    mounts.push("-v".to_string());
    mounts.push(format!("{}:/home/lal/volume", pwd.display()));

    trace!(" - mounting {}", git_cfg.display());
    trace!(" - mounting {}", pwd.display());

    if printonly {
        print!("docker run --rm");
        for m in mounts.clone() {
            print!(" {}", m);
        }
        print!(" -w /home/lal/volume --user lal {} {}",
            if interactive { "-it" } else { "-t" },
            &cfg.container,
        );
        for arg in command.clone() {
            print!(" {}", arg);
        }
        print!("\n");
    }
    else {
        let s = try!(Command::new("docker")
            .arg("run")
            .arg("--rm")
            .args(&mounts)
            .args(&vec!["-w", "/home/lal/volume"])
            .args(&vec!["--user", "lal"])
            .arg(if interactive { "-it" } else { "-t" })
            .arg(&cfg.container)
            .args(&command)
            .status());

        trace!("Exited docker");
        if !s.success() {
            return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
        }
    }
    Ok(())
}

/// Mounts and enters `.` in an interactive bash shell using the configured container.
pub fn shell(cfg: &Config, printonly: bool) -> LalResult<()> {
    if !printonly {
        info!("Entering docker container");
    }
    docker_run(&cfg, vec!["/bin/bash".to_string()], true, printonly)
}
