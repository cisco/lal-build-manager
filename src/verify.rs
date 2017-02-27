use walkdir::WalkDir;

use super::{Lockfile, Manifest, CliError, LalResult};


fn verify_sane_manifest(m: &Manifest) -> LalResult<()> {
    for (name, conf) in &m.components {
        // Verify ComponentSettings (manifest.components[x])
        debug!("Verifying component {}", name);
        if !conf.configurations.contains(&conf.defaultConfig) {
            let ename = format!("default configuration '{}' not found in configurations list",
                                conf.defaultConfig);
            return Err(CliError::InvalidBuildConfiguration(ename));
        }
    }
    Ok(())
}

fn verify_sane_input(m: &Manifest) -> LalResult<()> {
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
    // NB: deliberately not returning Err early because we want a large warning list
    // if INPUT folders are missing at the start of a build (forgot to fetch)
    for (d, v) in &m.dependencies {
        trace!("Verifying dependency from manifest: {}@{}", d, v);
        if !deps.contains(d) {
            warn!("Dependency {} not found in INPUT", d);
            error = Some(CliError::MissingDependencies);
        }
    }
    if let Some(e) = error { Err(e) } else { Ok(()) }
}

fn verify_global_versions(lf: &Lockfile, m: &Manifest) -> LalResult<()> {
    let all_deps = m.all_dependencies();
    for (name, dep) in &lf.dependencies {
        let v = dep.version
            .parse::<u32>()
            .map_err(|e| {
                debug!("Failed to parse first version of {} as int ({:?})", name, e);
                CliError::NonGlobalDependencies(name.clone())
            })?;
        // also ensure it matches the version in the manifest
        let vreq = *all_deps.get(name)
            .ok_or_else(|| {
                // This is a first level dependency - it should be in the manifest
                CliError::ExtraneousDependencies(name.clone())
            })?;
        if v != vreq {
            warn!("Dependency {} has version {}, but manifest requires {}",
                  name,
                  v,
                  vreq);
            return Err(CliError::InvalidVersion(name.clone()));
        }
    }
    Ok(())
}

fn verify_consistent_dependency_versions(lf: &Lockfile, m: &Manifest) -> LalResult<()> {
    for (name, vers) in lf.find_all_dependencies() {
        debug!("Found version(s) for {} as {:?}", name, vers);
        assert!(vers.len() > 0, "found versions");
        if vers.len() != 1 && m.dependencies.contains_key(&name) {
            warn!("Multiple version requirements on {} found in lockfile",
                  name.clone());
            return Err(CliError::MultipleVersions(name.clone()));
        }
    }
    Ok(())
}

fn verify_environment_consistency(lf: &Lockfile, env: &str) -> LalResult<()> {
    for (name, envs) in lf.find_all_environments() {
        debug!("Found environment(s) for {} as {:?}", name, envs);
        if envs.len() != 1 {
            warn!("Multiple environments used to build {}", name.clone());
            return Err(CliError::MultipleEnvironments(name.clone()));
        } else {
            let used_env = envs.iter().next().unwrap();
            if used_env != env {
                return Err(CliError::EnvironmentMismatch(name.clone(), used_env.clone()));
            }
        }
    }
    Ok(())
}

/// Verifies that `./INPUT` satisfies all strictness conditions.
///
/// This first verifies that there are no key mismatches between `defaultConfig` and
/// `configurations` in the manifest.
///
/// Once this is done, `INPUT` is analysed thoroughly via each components lockfiles.
/// Missing dependencies, or multiple versions dependend on implicitly are both
/// considered errors for verify, as are having custom versions in `./INPUT`.
///
/// This function is meant to be a helper for when we want official builds, but also
/// a way to tell developers that they are using things that differ from what jenkins
/// would use.
pub fn verify(m: &Manifest, env: &str) -> LalResult<()> {
    // 1. Verify that the manifest is sane
    verify_sane_manifest(m)?;

    // 2. dependencies in `INPUT` match `manifest.json`.
    if m.dependencies.is_empty() {
        // special case where lal fetch is not required and so INPUT may not exist
        // nothing needs to be verified in this case, so allow missing INPUT
        return Ok(());
    }
    verify_sane_input(m)?;

    // get data for big verify steps
    let lf = Lockfile::default().populate_from_input()?;

    // 3. verify the root level dependencies match the manifest
    verify_global_versions(&lf, m)?;

    // 4. the dependency tree is flat, and deps use only global deps
    verify_consistent_dependency_versions(&lf, m)?;

    // 5. verify all components are built in the same environment
    verify_environment_consistency(&lf, env)?;

    info!("Dependencies fully verified");
    Ok(())
}
