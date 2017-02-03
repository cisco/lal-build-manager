use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::collections::BTreeMap;
use serde_json;

use walkdir::WalkDir;

use init::Manifest;
use errors::{CliError, LalResult};

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
    let res = serde_json::from_str(&lock_str)?;
    Ok(res)
}

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
        let lck = read_partial_lockfile(&component)?;
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
        let actual_ver = deps.get(name).unwrap().clone();
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
