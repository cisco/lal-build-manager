use std::process::Command;
use std::env;
use std::path::Path;
use std::fs::File;
use std::fs;

use walkdir::WalkDir;

use configure::Config;
use shell;
use init::Manifest;
use errors::{LalResult, CliError};


fn tar_output(name: &str) -> LalResult<()> {
    use tar;
    use flate2::write::GzEncoder;
    use flate2::Compression;

    info!("Taring OUTPUT");

    let output = Path::new(&env::current_dir().unwrap()).join("OUTPUT");
    let tarsave = ["./", name, ".tar.gz"].concat();

    // pipe builder -> encoder -> file
    // creates a scope for mutable encoder reference (should be a better way though..)
    let file = try!(File::create(&tarsave));
    let mut encoder = GzEncoder::new(file, Compression::Default); // encoder writes file
    {
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
        try!(builder.finish());
    }
    try!(encoder.finish());

    // Replace OUTPUT with just the tarball
    try!(fs::remove_dir_all(&output));
    try!(fs::create_dir(&output));
    try!(fs::copy(&tarsave, output.join([name, ".tar.gz"].concat())));
    try!(fs::remove_file(&tarsave));

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
