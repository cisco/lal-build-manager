use std::path::Path;
use std::fs;
use std::io;
use std::process::Command;

use shell;
use verify::verify;
use super::{Lockfile, Manifest, Container, Config, LalResult, CliError, DockerRunFlags};

fn find_valid_build_script() -> LalResult<String> {
    use std::os::unix::fs::PermissionsExt;

    // less intrusive location for BUILD scripts
    let bpath_new = Path::new("./.lal/BUILD");
    let bpath_old = Path::new("./BUILD"); // fallback if new version does not exist
    let bpath = if bpath_new.exists() {
        if bpath_old.exists() {
            warn!("BUILD found in both .lal/ and current directory");
            warn!("Using the default: .lal/BUILD");
        }
        bpath_new
    }
    else {
        trace!("No BUILD existing in .lal");
        bpath_old
    };
    trace!("Using BUILD script found in {}", bpath.display());
    // Need the string to construct a list of argument for docker run
    // lossy convert because paths can somehow contain non-unicode?
    let build_string = bpath.to_string_lossy();

    // presumably we can always get the permissions of a file, right? (inb4 nfs..)
    let mode = bpath.metadata()?.permissions().mode();
    if mode & 0o111 == 0 {
        return Err(CliError::BuildScriptNotExecutable(build_string.into()));
    }
    Ok(build_string.into())
}


pub fn tar_output(tarball: &Path) -> LalResult<()> {
    info!("Taring OUTPUT");
    let mut args : Vec<String> = vec![
        "czf".into(),
        tarball.to_str().unwrap().into(), // path created internally - always valid unicode
        "--transform=s,^OUTPUT/,,".into(), // remove leading OUTPUT
    ];

    // Avoid depending on wildcards (which would also hide hidden files)
    // All links, hidden files, and regular files should go into the tarball.
    let findargs = vec!["OUTPUT/", "-type", "f", "-o", "-type", "l"];
    debug!("find {}", findargs.join(" "));
    let find_output = Command::new("find").args(&findargs).output()?;
    let find_str = String::from_utf8_lossy(&find_output.stdout);

    // append each file as an arg to the main tar process
    for f in find_str.trim().split('\n') {
        args.push(f.into())
    }

    // basically `tar czf component.tar --transform.. $(find OUTPUT -type f -o -type l)`:
    debug!("tar {}", args.join(" "));
    let s = Command::new("tar").args(&args).status()?;

    if !s.success() {
        return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
    }
    Ok(())
}

fn ensure_dir_exists_fresh(subdir: &str) -> io::Result<()> {
    let dir = Path::new(".").join(subdir);
    if dir.is_dir() {
        // clean it out first
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir(&dir)?;
    Ok(())
}

/// Helper to print the buildable components from the `Manifest`
pub fn build_list(manifest: &Manifest) -> LalResult<()> {
    for k in manifest.components.keys() {
        println!("{}", k);
    }
    Ok(())
}

/// Helper to print the available configurations for a buildable Component
pub fn configuration_list(component: &str, manifest: &Manifest) -> LalResult<()> {
     let component_settings = match manifest.components.get(component) {
        Some(c) => c,
        None => return Ok(()), // invalid component - but this is for completion
    };
    for c in &component_settings.configurations {
        println!("{}", c);
    }
    Ok(())
}

/// Configurable build flags for `lal build`
pub struct BuildOptions {
    /// Component to build if specified
    pub name: Option<String>,
    /// Configuration to use for the component if specified
    pub configuration: Option<String>,
    /// Container to run the `./BUILD` script in
    pub container: Container,
    /// Create release tarball in `./ARTIFACT`
    pub release: bool,
    /// An explicit version to put in the lockfile
    pub version: Option<String>,
    /// An explicit sha changeset id to put in the lockfile
    pub sha: Option<String>,
    /// Ignore verify failures
    pub force: bool,
}


/// Runs the `./BUILD` script in a container and packages artifacts.
///
/// The function performs basic sanity checks, before shelling out to `docker run`
/// to perform the actual execution of the containerized `./BUILD` script.
///
pub fn build(cfg: &Config,
             manifest: &Manifest,
             opts: BuildOptions,
             envname: String,
             printonly: bool)
             -> LalResult<()> {
    // have a better warning on first file-io operation
    // if nfs mounts and stuff cause issues this usually catches it
    ensure_dir_exists_fresh("OUTPUT").map_err(|e| {
            error!("Failed to clean out OUTPUT dir: {}", e);
            e
        })?;

    debug!("Version flag is {:?}", opts.version);

    // Verify INPUT
    let mut verify_failed = false;
    if let Some(e) = verify(manifest, &envname).err() {
        if !opts.force {
            return Err(e);
        }
        verify_failed = true;
        warn!("Verify failed - build will fail on jenkins, but continuing");
    }


    let component = opts.name.unwrap_or_else(|| manifest.name.clone());
    debug!("Getting configurations for {}", component);

    // find component details in components.NAME
    let component_settings = match manifest.components.get(&component) {
        Some(c) => c,
        None => return Err(CliError::MissingComponent(component)),
    };
    let configuration_name: String = if let Some(c) = opts.configuration {
        c.to_string()
    } else {
        component_settings.defaultConfig.clone()
    };
    if !component_settings.configurations.contains(&configuration_name) {
        let ename = format!("{} not found in configurations list", configuration_name);
        return Err(CliError::InvalidBuildConfiguration(ename));
    }
    let lockfile = try!(Lockfile::new(&component,
                                      &opts.container,
                                      &envname,
                                      opts.version,
                                      Some(&configuration_name))
        .set_default_env(manifest.environment.clone())
        .attach_revision_id(opts.sha)
        .populate_from_input());

    let lockpth = Path::new("./OUTPUT/lockfile.json");
    lockfile.write(lockpth, true)?; // always put a lockfile in OUTPUT at the start of a build

    let bpath = find_valid_build_script()?;
    let cmd = vec![bpath, component.clone(), configuration_name];

    debug!("Build script is {:?}", cmd);
    if !printonly {
        info!("Running build script in {} container", envname);
    }

    let run_flags = DockerRunFlags {
        interactive: cfg.interactive,
        privileged: false,
    };
    shell::docker_run(cfg, &opts.container, cmd, run_flags, printonly)?;

    // Extra info and warnings for people who missed the leading ones (build is spammy)
    if verify_failed {
        warn!("Build succeeded - but `lal verify` failed");
        warn!("Please make sure you are using correct dependencies before pushing")
    } else {
        info!("Build succeeded with verified dependencies")
    }
    // environment is temporarily optional in manifest:
    if let Some(ref mandated_env) = manifest.environment {
        if &envname != mandated_env {
            // default was set, and we used not that
            warn!("Build was using non-default {} environment", envname);
        }
    } else {
        // default was not set, impossible to tell if this was sane
        warn!("Build was done using non-default {} environment", envname);
        warn!("Please hardcode an environment inside manifest.json");
    }

    if opts.release && !printonly {
        trace!("Create ARTIFACT dir");
        ensure_dir_exists_fresh("ARTIFACT")?;
        trace!("Copy lockfile to ARTIFACT dir");
        fs::copy(&lockpth, Path::new("./ARTIFACT/lockfile.json"))?;

        trace!("Tar up OUTPUT into ARTIFACT/component.tar.gz");
        let tarpth = Path::new("./ARTIFACT").join([component, ".tar.gz".into()].concat());
        tar_output(&tarpth)?;
    }
    Ok(())
}
