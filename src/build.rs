use std::path::Path;
use std::fs;

use shell;
use verify::verify;
use super::{ensure_dir_exists_fresh, output, Lockfile, Manifest, Container, Config, LalResult,
            CliError, DockerRunFlags, ShellModes};


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
    } else if bpath_old.exists() {
        bpath_old
    } else {
        return Err(CliError::MissingBuildScript);
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
    /// Use the `simple` verify algorithm
    pub simple_verify: bool,
}


/// Runs the `./BUILD` script in a container and packages artifacts.
///
/// The function performs basic sanity checks, before shelling out to `docker run`
/// to perform the actual execution of the containerized `./BUILD` script.
///
pub fn build(
    cfg: &Config,
    manifest: &Manifest,
    opts: &BuildOptions,
    envname: String,
    _modes: ShellModes,
) -> LalResult<()> {
    let mut modes = _modes;

    // have a better warning on first file-io operation
    // if nfs mounts and stuff cause issues this usually catches it
    ensure_dir_exists_fresh("./OUTPUT")
        .map_err(|e| {
            error!("Failed to clean out OUTPUT dir: {}", e);
            e
        })?;

    debug!("Version flag is {:?}", opts.version);

    // Verify INPUT
    let mut verify_failed = false;
    if let Some(e) = verify(manifest, &envname, opts.simple_verify).err() {
        if !opts.force {
            return Err(e);
        }
        verify_failed = true;
        warn!("Verify failed - build will fail on jenkins, but continuing");
    }


    let component = opts.name.clone().unwrap_or_else(|| manifest.name.clone());
    debug!("Getting configurations for {}", component);

    // A couple of matchups of configurations and components and sanity checks
    // If verify passed then these won't fail, but verify is sometimes ignorable

    // find component details in components.NAME
    let component_settings = match manifest.components.get(&component) {
        Some(c) => c,
        None => return Err(CliError::MissingComponent(component)),
    };
    let configuration_name: String = if let Some(c) = opts.configuration.clone() {
        c
    } else {
        component_settings.defaultConfig.clone()
    };
    if !component_settings.configurations.contains(&configuration_name) {
        let ename = format!("{} not found in configurations list", configuration_name);
        return Err(CliError::InvalidBuildConfiguration(ename));
    }
    let lockfile = Lockfile::new(&component,
                                 &opts.container,
                                 &envname,
                                 opts.version.clone(),
                                 Some(&configuration_name))
        .set_default_env(manifest.environment.clone())
        .attach_revision_id(opts.sha.clone())
        .populate_from_input()?;

    let lockpth = Path::new("./OUTPUT/lockfile.json");
    lockfile.write(lockpth)?; // always put a lockfile in OUTPUT at the start of a build

    let bpath = find_valid_build_script()?;
    let cmd = vec![bpath, component.clone(), configuration_name];

    if let Some(v) = opts.version.clone() {
        modes.env_vars.push(format!("BUILD_VERSION={}", v));
    }

    debug!("Build script is {:?}", cmd);
    if !modes.printonly {
        info!("Running build script in {} container", envname);
    }

    let run_flags = DockerRunFlags {
        interactive: cfg.interactive,
        privileged: false,
    };
    shell::docker_run(cfg, &opts.container, cmd, &run_flags, &modes)?;
    if modes.printonly {
        return Ok(()); // nothing else worth doing - warnings are pointless
    }

    // Extra info and warnings for people who missed the leading ones (build is spammy)
    if verify_failed {
        warn!("Build succeeded - but `lal verify` failed");
        warn!("Please make sure you are using correct dependencies before pushing")
    } else {
        info!("Build succeeded with verified dependencies")
    }
    // environment is temporarily optional in manifest:
    if envname != manifest.environment {
        warn!("Build was using non-default {} environment", envname);
    }

    if opts.release && !modes.printonly {
        trace!("Create ARTIFACT dir");
        ensure_dir_exists_fresh("./ARTIFACT")?;
        trace!("Copy lockfile to ARTIFACT dir");
        fs::copy(&lockpth, Path::new("./ARTIFACT/lockfile.json"))?;

        trace!("Tar up OUTPUT into ARTIFACT/component.tar.gz");
        let tarpth = Path::new("./ARTIFACT").join([component, ".tar.gz".into()].concat());
        output::tar(&tarpth)?;
    }
    Ok(())
}
