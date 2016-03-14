use std::env;
use std::path::Path;
use std::fs::File;
use std::fs;
use std::io;

use walkdir::WalkDir;

use configure::Config;
use shell;
use init::Manifest;
use errors::{LalResult, CliError};
use lockfile;


fn tar_output(tarball: &Path) -> LalResult<()> {
    use tar;
    use flate2::write::GzEncoder;
    use flate2::Compression;

    info!("Taring OUTPUT");

    // pipe builder -> encoder -> file
    let file = try!(File::create(&tarball));
    let mut encoder = GzEncoder::new(file, Compression::Default); // encoder writes file
    let mut builder = tar::Builder::new(&mut encoder); // tar builder writes to encoder

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
    // builder, THEN encoder, are finish()d at the end of this scope
    // tarball has not been completely written until this function is over

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

pub fn build(cfg: &Config, manifest: &Manifest, name: Option<&str>) -> LalResult<()> {
    try!(ensure_dir_exists_fresh("OUTPUT"));

    // TODO: generate lockfile
    info!("Running build script in docker container");
    let component = name.unwrap_or(&manifest.name);
    // TODO: build flags
    let cmd = vec!["./BUILD", &component, &cfg.target];
    debug!("Build script is {:?}", cmd);
    try!(shell::docker_run(&cfg, cmd, false));

    try!(ensure_dir_exists_fresh("ARTIFACT"));

    let cwd = try!(env::current_dir());
    let pwd = Path::new(&cwd);
    let tarball = pwd.join(["./", component, ".tar.gz"].concat());
    try!(tar_output(&tarball));

    try!(fs::copy(&tarball, pwd.join("ARTIFACT").join([component, ".tar.gz"].concat())));
    try!(lockfile::generate(manifest));
    Ok(())
}
