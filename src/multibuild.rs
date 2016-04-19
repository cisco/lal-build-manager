use walkdir::WalkDir;

//use build;
use {LalResult, Manifest, Config};
// need CliError and custom error types implemented in it:
// - cyclic dependency error (makes no sense to build)
// - unconnected dependencies error (these are not built together)
// - dependencies not found error (unless we find a way to deal with this)

/// Build multiple components from subdirectories sequentially
pub fn multibuild(cfg: &Config, components: Vec<&str>) -> LalResult<()> {
    let dirs = WalkDir::new(".")
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| e.path().join("manifest.json").is_file());

    debug!("In multibuild");
    for d in dirs {
        let pth = d.path();
        debug!("checking {}", pth.to_str().unwrap());
        let mf = try!(Manifest::read_from(pth.to_path_buf()));
        debug!("Found manifest for {} in a subfolder", mf.name);
    }

    // TODO: figure out an order
    // for i in order { build, stash as multibuild, update into next }
    // build with default configuration, in non-release mode + everything else off
    // This is the non-trivial bit

    // NB: build likely needs to become directory aware now
    //build(cfg, mf, None, false, None, false)

    unimplemented!()
}
