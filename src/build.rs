use std::env;
use std::path::Path;
use std::fs::File;
use std::fs;
use std::io;

use walkdir::WalkDir;

use configure::Config;
use shell;
use init::Manifest;
use verify::verify;
use errors::{LalResult, CliError};
use util::lockfile::Lock;


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
    let cwd = try!(env::current_dir());
    let pwd = Path::new(&cwd);
    let dir = pwd.join(subdir);
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
             release: bool,
             version: Option<&str>)
             -> LalResult<()> {
    try!(ensure_dir_exists_fresh("OUTPUT"));

    debug!("Version flag is {}", version.unwrap_or("unset"));
    let lockfile = try!(Lock::new(&manifest.name, version).populate_from_input());

    // Verify INPUT
    if let Some(e) = verify(manifest.clone()).err() {
        if version.is_some() {
            return Err(e);
        }
        warn!("Verify failed - build will fail on jenkins, but continuing");
    }

    info!("Running build script in docker container");
    let component = name.unwrap_or(&manifest.name);
    // TODO: build flags
    let cmd = vec!["./BUILD", &component];
    debug!("Build script is {:?}", cmd);
    try!(shell::docker_run(&cfg, cmd, false));

    if release {
        try!(ensure_dir_exists_fresh("ARTIFACT"));

        let cwd = try!(env::current_dir());
        let pwd = Path::new(&cwd);
        let tarball = pwd.join(["./", component, ".tar.gz"].concat());
        try!(tar_output(&tarball));

        try!(fs::copy(&tarball,
                      pwd.join("ARTIFACT").join([component, ".tar.gz"].concat())));
        try!(fs::remove_file(&tarball));

        let lockpath = pwd.join("ARTIFACT").join("lockfile.json");
        try!(lockfile.write(&lockpath));
    }
    Ok(())
}
