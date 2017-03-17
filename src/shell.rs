use std::process::Command;
use std::env;
use std::path::Path;
use std::vec::Vec;

use super::{Config, Container, CliError, LalResult};

/// Verifies that `id -u` and `id -g` are both 1000
///
/// Docker user namespaces are not properly supported by our setup,
/// so for builds to work sanely, user ids and group ids should match a standard
/// linux setup and in particular, match the first user in a normal container.
fn permission_sanity_check() -> LalResult<()> {
    let uid_output = Command::new("id").arg("-u").output()?;
    let uid_str = String::from_utf8_lossy(&uid_output.stdout);
    let uid = uid_str.trim().parse::<u32>().unwrap(); // trust `id -u` is sane
    if uid != 1000 {
        return Err(CliError::DockerPermissionSafety(format!("UID is {}, not 1000", uid)));
    }

    let gid_output = Command::new("id").arg("-g").output()?;
    let gid_str = String::from_utf8_lossy(&gid_output.stdout);
    let gid = gid_str.trim().parse::<u32>().unwrap(); // trust `id -g` is sane
    if gid != 1000 {
        return Err(CliError::DockerPermissionSafety(format!("GID is {}, not 1000", gid)));
    }

    Ok(())
}

/// Flags for docker run that vary for different use cases
///
/// `interactive` should be on by default, but machine accounts should turn this off
/// `privileged` is needed on some setups for `gdb` and other low level tools to work
///
/// NB: The derived default should only be used by tests (all false/zero)
#[derive(Default)]
pub struct DockerRunFlags {
    /// Pass --interactive (allows ctrl-c on builds/scripts/shell commands)
    pub interactive: bool,
    /// Pass --privileged (situational)
    pub privileged: bool,
}


/// Runs an arbitrary command in the configured docker environment
///
/// This will mount the current directory as `~/volume` as well as a few conveniences,
/// and absorb the `Stdio` supplied by this `Command`.
///
/// This is the most general function, used by both `lal build` and `lal shell`.
pub fn docker_run(cfg: &Config,
                  container: &Container,
                  command: Vec<String>,
                  flags: &DockerRunFlags,
                  modes: &ShellModes)
                  -> LalResult<()> {
    trace!("Finding home and cwd");
    let home = env::home_dir().unwrap(); // crash if no $HOME
    let git_cfg = Path::new(&home).join(".gitconfig");
    let pwd = env::current_dir().unwrap();

    // construct arguments vector
    let mut args: Vec<String> = vec!["run".into(), "--rm".into()];
    for mount in cfg.mounts.clone() {
        trace!(" - mounting {}", mount.src);
        args.push("-v".into());
        let mnt = format!("{}:{}{}",
                          mount.src,
                          mount.dest,
                          if mount.readonly { ":ro" } else { "" });
        args.push(mnt);
    }
    trace!(" - mounting {}", git_cfg.display());
    trace!(" - mounting {}", pwd.display());
    args.push("-v".into());
    args.push(format!("{}:/home/lal/.gitconfig:ro", git_cfg.display()));
    args.push("-v".into());
    args.push(format!("{}:/home/lal/volume", pwd.display()));

    // X11 forwarding
    if modes.x11_forwarding {
        // requires calling `xhost local:docker` first
        args.push("-v".into());
        args.push("/tmp/.X11-unix:/tmp/.X11-unix:ro".into());
        args.push("--env=DISPLAY".into());
        args.push("-v".into());
        // xauth also needed for `ssh -X` through `lal -X`
        args.push(format!("{}/.Xauthority:/home/lal/.Xauthority:ro", home.display()));
        // QT compat
         args.push("--env=QT_X11_NO_MITSHM=1".into());
    }
    if modes.host_networking {
        // also needed for for `ssh -X` into `lal -X`
        args.push("--net=host".into());
    }


    if flags.privileged {
        args.push("--privileged".into())
    }

    args.push("-w".into());
    args.push("/home/lal/volume".into());
    args.push("--user".into());
    args.push("lal".into());

    // If no command, then override entrypoint to /bin/bash
    // This happens when we use `lal shell` without args
    if command.is_empty() {
        args.push("--entrypoint".into());
        args.push("/bin/bash".into());
    }
    args.push((if flags.interactive { "-it" } else { "-t" }).into());

    args.push(format!("{}:{}", container.name, container.tag));
    for c in command {
        args.push(c);
    }

    // run or print docker command
    if modes.printonly {
        print!("docker");
        for arg in args {
            if arg.contains(" ") {
                // leave quoted args quoted
                print!(" \"{}\"", arg);
            } else {
                print!(" {}", arg);
            }
        }
        println!("");
    } else {
        trace!("Performing docker permission sanity check");
        let _ = permission_sanity_check().map_err(|e| {
            warn!("{}", e);
            warn!("You will likely have permission issues");
        }); // keep going, but with a warning if it failed
        trace!("Permissions verified, entering docker");
        let s = Command::new("docker").args(&args).status()?;
        trace!("Exited docker");
        if !s.success() {
            return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
        }
    }
    Ok(())
}

/// Various ways to invoke `docker_run`
#[derive(Default)]
pub struct ShellModes {
    /// Just print the command used rather than do it
    pub printonly: bool,
    /// Attempt to forward the X11 socket and all it needs
    pub x11_forwarding: bool,
    /// Host networking
    pub host_networking: bool,
}



/// Mounts and enters `.` in an interactive bash shell using the configured container.
///
/// If a command vector is given, this is called non-interactively instead of /bin/bash
/// You can thus do `lal shell ./BUILD target` or ``lal shell bash -c "cmd1; cmd2"`
pub fn shell(cfg: &Config,
             container: &Container,
             modes: &ShellModes,
             cmd: Option<Vec<&str>>,
             privileged: bool)
             -> LalResult<()> {
    if !modes.printonly {
        info!("Entering {}", container);
    }
    let flags = DockerRunFlags {
        interactive: cmd.is_none() || cfg.interactive,
        privileged: privileged,
    };
    let mut bash = vec![];
    if let Some(cmdu) = cmd {
        for c in cmdu {
            bash.push(c.to_string())
        }
    }
    docker_run(cfg, container, bash, &flags, modes)
}

/// Runs a script in `.lal/scripts/` with supplied arguments in a docker shell
///
/// This is a convenience helper for running things that aren't builds.
/// E.g. `lal run my-large-test RUNONLY=foo`
pub fn script(cfg: &Config,
              container: &Container,
              name: &str,
              args: Vec<&str>,
              modes: &ShellModes,
              privileged: bool)
              -> LalResult<()> {
    let pth = Path::new(".").join(".lal").join("scripts").join(&name);
    if !pth.exists() {
        return Err(CliError::MissingScript(name.into()));
    }

    let flags = DockerRunFlags {
        interactive: cfg.interactive,
        privileged: privileged,
    };

    // Simply run the script by adding on the arguments
    let cmd = vec!["bash".into(),
                   "-c".into(),
                   format!("source {}; main {}", pth.display(), args.join(" "))];
    Ok(docker_run(cfg, container, cmd, &flags, modes)?)
}
