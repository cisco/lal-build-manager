#![allow(missing_docs)]

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::collections::BTreeMap;
use serde_json;

use walkdir::WalkDir;

use super::{Manifest, Lockfile, CliError, LalResult};

#[derive(Deserialize)]
struct PartialLock {
    pub version: String,
}
fn read_partial_lockfile(component: &str) -> LalResult<PartialLock> {
    let lock_path = Path::new("./INPUT").join(component).join("lockfile.json");
    if !lock_path.exists() {
        return Err(CliError::MissingLockfile(component.to_string()));
    }
    let mut lock_str = String::new();
    trace!("Deserializing lockfile for {}", component);
    File::open(&lock_path)?.read_to_string(&mut lock_str)?;
    Ok(serde_json::from_str(&lock_str)?)
}

pub fn present() -> bool {
    Path::new("./INPUT").is_dir()
}

/// Simple INPUT analyzer for the lockfile generator and `analyze_full`
pub fn analyze() -> LalResult<BTreeMap<String, String>> {
    let input = Path::new("./INPUT");

    let mut deps = BTreeMap::new();
    if !input.is_dir() {
        return Ok(deps);
    }
    let dirs = WalkDir::new("INPUT")
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir());

    for d in dirs {
        let pth = d.path().strip_prefix("INPUT").unwrap();
        let component = pth.to_str().unwrap();
        let lck = read_partial_lockfile(component)?;
        deps.insert(component.to_string(), lck.version);
    }
    Ok(deps)
}

#[derive(Debug)]
pub struct InputDependency {
    pub name: String,
    pub missing: bool,
    pub extraneous: bool,
    pub development: bool,
    pub version: String, // on disk
    pub requirement: Option<String>, // from manifest
}

pub type InputMap = BTreeMap<String, InputDependency>;

/// Helper for `lal::status`
pub fn analyze_full(manifest: &Manifest) -> LalResult<InputMap> {
    let input = Path::new("./INPUT");

    let deps = analyze()?;
    let saved_deps = manifest.all_dependencies();

    let mut depmap = InputMap::new();
    if !input.is_dir() {
        return Ok(depmap);
    }

    // check manifested deps
    // something in manifest
    for (d, v) in saved_deps.clone() {
        // use manifest ver if not in INPUT
        let version: String = match deps.get(&d) {
            Some(v) => v.clone(),
            None => v.to_string(),
        };
        depmap.insert(d.clone(),
                      InputDependency {
                          name: d.clone(),
                          version: version,
                          requirement: Some(format!("{}", v)),
                          missing: deps.get(&d).is_none(),
                          development: manifest.devDependencies.contains_key(&d),
                          extraneous: false,
                      });
    }
    // check for potentially non-manifested deps
    // i.e. something in INPUT, but not in manifest
    for name in deps.keys() {
        let actual_ver = deps[name].clone();
        if !saved_deps.contains_key(name) {
            depmap.insert(name.clone(),
                          InputDependency {
                              name: name.clone(),
                              version: actual_ver,
                              requirement: None,
                              missing: false,
                              development: false,
                              extraneous: true,
                          });
        }
    }

    Ok(depmap)
}

/// Basic part of input verifier - checks that everything is at least present
pub fn verify_dependencies_present(m: &Manifest) -> LalResult<()> {
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

/// Optional part of input verifier - checks that all versions use correct versions
pub fn verify_global_versions(lf: &Lockfile, m: &Manifest) -> LalResult<()> {
    let all_deps = m.all_dependencies();
    for (name, dep) in &lf.dependencies {
        let v = dep.version
            .parse::<u32>()
            .map_err(|e| {
                debug!("Failed to parse first version of {} as int ({:?})", name, e);
                CliError::NonGlobalDependencies(name.clone())
            })?;
        // also ensure it matches the version in the manifest
        let vreq = *all_deps
            .get(name)
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
        // Prevent Cycles (enough to stop it at one manifest level)
        if &m.name == name {
            return Err(CliError::DependencyCycle(name.clone()));
        }
    }
    Ok(())
}

/// Strict requirement for verifier - dependency tree must be flat-equivalent
pub fn verify_consistent_dependency_versions(lf: &Lockfile, m: &Manifest) -> LalResult<()> {
    for (name, vers) in lf.find_all_dependency_versions() {
        debug!("Found version(s) for {} as {:?}", name, vers);
        assert!(!vers.is_empty(), "found versions");
        if vers.len() != 1 && m.dependencies.contains_key(&name) {
            warn!("Multiple version requirements on {} found in lockfile",
                  name.clone());
            warn!("If you are trying to propagate {0} into the tree, \
                    you need to follow `lal propagate {0}`",
                  name);
            return Err(CliError::MultipleVersions(name.clone()));
        }
    }
    Ok(())
}

/// Strict requirement for verifier - all deps must be built in same environment
pub fn verify_environment_consistency(lf: &Lockfile, env: &str) -> LalResult<()> {
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
