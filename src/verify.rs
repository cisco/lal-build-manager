use walkdir::WalkDir;

use {Lockfile, Manifest, CliError, LalResult};
use util::lockfile::find_all_dependencies;

pub fn verify(m: Manifest) -> LalResult<()> {
    // 1. Verify that the manifest is sane
    for (name, conf) in m.components {
        // Verify ComponentSettings (manifest.components[x])
        debug!("Verifying component {}", name);
        if !conf.configurations.contains(&conf.defaultConfig) {
            let ename = format!("default configuration '{}' not found in configurations list", conf.defaultConfig);
            return Err(CliError::InvalidBuildConfiguration(ename));
        }
    }

    // 2. dependencies in `INPUT` match `manifest.json`.
    if m.dependencies.len() == 0 {
        return Ok(()); // nothing to verify - so accept a missing directory
    }

    let mut error = None;
    let mut deps = vec![];
    let dirs = WalkDir::new("INPUT")
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir());
    for entry in dirs {
        let pth = entry.path().strip_prefix("INPUT").unwrap();
        debug!("-> {}", pth.display());

        let component = pth.to_str().unwrap();
        deps.push(component.to_string());
    }
    debug!("Found the following deps in INPUT: {:?}", deps);
    for (d, _) in m.dependencies {
        debug!("Verifying dependency from manifest: {}", d);
        if !deps.contains(&d) {
            warn!("Dependency {} not found in INPUT", d);
            error = Some(CliError::MissingDependencies);
        }
    }

    // 3. the dependency tree is flat and only global dependencies found
    debug!("Reading all lockfiles");
    let lf = try!(Lockfile::new("templock", None, None).populate_from_input());
    let dep_usage = find_all_dependencies(&lf);
    for (name, vers) in dep_usage {
        debug!("Found version(s) for {} as {:?}", name, vers);
        if vers.len() != 1 {
            error = Some(CliError::MultipleVersions(name.clone()));
            // TODO: should have better way to allow user to debug here..
        }
        assert!(vers.len() > 0, "found versions");
        // if version cannot be parsed as an int, it's not a global dependency
        if let Err(e) = vers.iter().next().unwrap().parse::<u32>() {
            debug!("Failed to parse first version of {} as int ({:?})", name, e);
            error = Some(CliError::NonGlobalDependencies(name.clone()));
        }

    }

    // Return one of the errors as the main one (no need to vectorize these..)
    if error.is_some() {
        return Err(error.unwrap());
    }
    info!("Dependencies fully verified");
    Ok(())
}
