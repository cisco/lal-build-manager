use std::process::Command;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::fs;

use walkdir::WalkDir;

use configure::Config;
use shell;
use init::Manifest;
use errors::LalResult;


fn tar_output(name: &str) -> LalResult<()> {
    use tar::{Archive, Builder};
    //use flate2::read::GzEncoder;
    //use flate2::Compression;
    info!("Taring OUTPUT");

    let output = Path::new(&env::current_dir().unwrap()).join("OUTPUT");
    let dest = ["./", name, ".tar"].concat();

    let file = try!(File::create(&dest));
    let mut a = Builder::new(file);
    //let a = Archive::new(GzEncoder::new(file, Compression::Default));


    for entry in WalkDir::new("OUTPUT").min_depth(1).into_iter().filter_map(|e| e.ok()) {
        let pth = entry.path().strip_prefix("OUTPUT").unwrap();
        debug!("-> {}", pth.display());
        let mut f = try!(File::open(entry.path()));
        if entry.path().is_dir() {
            // can ignore this, but this allows empty directories
            try!(a.append_dir(pth, entry.path()));
        }
        else {
            try!(a.append_file(pth, &mut f));
        }
    }
    try!(a.finish());

    Ok(())
}

pub fn build(cfg: &Config, manifest: &Manifest, name: Option<&str>) -> LalResult<()> {
    try!(Command::new("mkdir").arg("-p").arg("OUTPUT").output());

    // TODO: generate lockfile
    info!("Running build script in docker container");
    let component = name.unwrap_or(&manifest.name);
    // TODO: build flags
    let cmd = vec!["./BUILD", &component, &cfg.target];
    debug!("Build script is {:?}", cmd);
    try!(shell::docker_run(&cfg, cmd, false));

    try!(tar_output(&component));
    Ok(())
}
