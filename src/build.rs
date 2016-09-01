use std::path::Path;
use std::fs::File;
use std::fs;
use std::io;
// use std::collections::HashMap;

use walkdir::WalkDir;

use shell;
use verify::verify;
use {Lockfile, Manifest, Container, Config, LalResult, CliError};

pub fn tar_output(tarball: &Path) -> LalResult<()> {
    use tar;
    use flate2::write::GzEncoder;
    use flate2::Compression;

    info!("Taring OUTPUT");

    // pipe builder -> encoder -> file
    let file = try!(File::create(&tarball));
    let mut encoder = GzEncoder::new(file, Compression::Default); // encoder writes file
    let mut builder = tar::Builder::new(&mut encoder); // tar builder writes to encoder
    // builder, THEN encoder, are finish()d at the end of this scope
    // tarball has not been completely written until this function is over

    let files = WalkDir::new("OUTPUT")
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.path().is_dir()); // ignore directories (these are created anyway)
    // Last line means that we exclude empty directories (must be added manually with tar)

    let mut had_files = false;
    // add files to builder
    for f in files {
        let pth = f.path().strip_prefix("OUTPUT").unwrap();
        debug!("-> {}", pth.display());
        let mut f = try!(File::open(f.path()));
        try!(builder.append_file(pth, &mut f));
        had_files = true;
    }
    if !had_files {
        return Err(CliError::MissingBuild);
    }
    Ok(())
}

fn ensure_dir_exists_fresh(subdir: &str) -> io::Result<()> {
    let dir = Path::new(".").join(subdir);
    if dir.is_dir() {
        // clean it out first
        try!(fs::remove_dir_all(&dir));
    }
    try!(fs::create_dir(&dir));
    Ok(())
}

/// Helper to print the buildable components from the `Manifest`
pub fn build_list(manifest: &Manifest) -> LalResult<()> {
    for k in manifest.components.keys() {
        println!("{}", k);
    }
    Ok(())
}

/// Runs the `./BUILD` script in a container and packages artifacts.
///
/// Expects a pre-read `Manifest` file, a `Config` file, as well as a bunch of optional flags
/// that the user may supply..
///
/// The function performs basic sanity checks, before shelling out to `docker run`
/// to perform the actual execution of the containerized `./BUILD` script.
///
/// In release mode, tarballs and lockfiles are created in `./ARTIFACT/`.
pub fn build(cfg: &Config,
             manifest: &Manifest,
             name: Option<&str>,
             configuration: Option<&str>,
             release: bool,
             version: Option<&str>,
             strict: bool,
             container: &Container,
             envname: String,
             printonly: bool)
             -> LalResult<()> {
    // have a better warning on first file-io operation
    // if nfs mounts and stuff cause issues this usually catches it
    try!(ensure_dir_exists_fresh("OUTPUT").map_err(|e| {
        error!("Failed to clean out OUTPUT dir: {}", e);
        e
    }));

    debug!("Version flag is {}", version.unwrap_or("unset"));

    // Verify INPUT
    if let Some(e) = verify(manifest, &envname).err() {
        if version.is_some() || strict {
            return Err(e);
        }
        warn!("Verify failed - build will fail on jenkins, but continuing");
    }

    let component = name.unwrap_or(&manifest.name);
    debug!("Getting configurations for {}", component);

    // find component details in components.NAME
    let component_settings = match manifest.components.get(component) {
        Some(c) => c,
        None => return Err(CliError::MissingComponent(component.to_string())),
    };
    let configuration_name: String = if let Some(c) = configuration {
        c.to_string()
    } else {
        component_settings.defaultConfig.clone()
    };
    if !component_settings.configurations.contains(&configuration_name) {
        let ename = format!("{} not found in configurations list", configuration_name);
        return Err(CliError::InvalidBuildConfiguration(ename));
    }
    let lockfile = try!(Lockfile::new(&manifest.name,
                                      container,
                                      &envname,
                                      version,
                                      Some(&configuration_name))
        .populate_from_input());
    let lockpth = Path::new("./OUTPUT/lockfile.json");
    try!(lockfile.write(&lockpth, true)); // always put a lockfile in OUTPUT at the start of a build

    let cmd = vec!["./BUILD".into(), component.into(), configuration_name];

    debug!("Build script is {:?}", cmd);
    if !printonly {
        info!("Running build script in {} container", envname);
    }

    try!(shell::docker_run(cfg, container, cmd, false, printonly, false));

    if release && !printonly {
        trace!("Create ARTIFACT dir");
        try!(ensure_dir_exists_fresh("ARTIFACT"));
        trace!("Copy lockfile to ARTIFACT dir");
        try!(fs::copy(&lockpth, Path::new("./ARTIFACT/lockfile.json")));

        trace!("Tar up OUTPUT into ARTIFACT/component.tar.gz");
        let tarpth = Path::new("./ARTIFACT").join([component, ".tar.gz"].concat());
        try!(tar_output(&tarpth));
    }
    Ok(())
}
