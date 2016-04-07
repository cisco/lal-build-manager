use std::path::Path;
use std::fs::File;
use std::fs;
use std::io;
// use std::collections::HashMap;

use walkdir::WalkDir;

use shell;
use verify::verify;
use {Lockfile, Manifest, Config, LalResult, CliError};

fn tar_output(tarball: &Path) -> LalResult<()> {
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

pub fn build(cfg: &Config,
             manifest: &Manifest,
             name: Option<&str>,
             configuration: Option<&str>,
             release: bool,
             version: Option<&str>)
             -> LalResult<()> {
    try!(ensure_dir_exists_fresh("OUTPUT"));

    debug!("Version flag is {}", version.unwrap_or("unset"));

    // Verify INPUT
    if let Some(e) = verify(manifest.clone()).err() {
        if version.is_some() {
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
                                      &cfg.container,
                                      version,
                                      Some(&configuration_name))
        .populate_from_input());

    let cmd = vec!["./BUILD".to_string(), component.to_string(), configuration_name];

    debug!("Build script is {:?}", cmd);
    info!("Running build script in docker container");
    try!(shell::docker_run(&cfg, cmd, false));

    if release {
        try!(ensure_dir_exists_fresh("ARTIFACT"));
        // Save lockfile in both ARTIFACT and OUTPUT (so it's also in the archive)
        let lockpth = Path::new("./OUTPUT/lockfile.json");
        try!(lockfile.write(&lockpth));
        try!(fs::copy(&lockpth, Path::new("./ARTIFACT/lockfile.json")));

        // Tar up OUTPUT into ARTIFACT/component.tar.gz
        let tarpth = Path::new("./ARTIFACT").join([component, ".tar.gz"].concat());
        try!(tar_output(&tarpth));
    }
    Ok(())
}
