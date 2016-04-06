use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use rustc_serialize::json;

use walkdir::WalkDir;

use init::Manifest;
use errors::{CliError, LalResult};

#[derive(RustcDecodable)]
struct PartialLock {
  pub version: String,
}
fn read_partial_lockfile(component: &str) -> LalResult<PartialLock> {
    let lock_path = Path::new("./INPUT").join(component).join("lockfile.json");
    if ! lock_path.exists() {
        return Err(CliError::MissingLockfile(component.to_string()));
    }
    let mut lock_str = String::new();
    try!(try!(File::open(&lock_path)).read_to_string(&mut lock_str));
    let res = try!(json::decode(&lock_str));
    Ok(res)
}

pub fn analyze() -> LalResult<HashMap<String, String>> {
    let input = Path::new("./INPUT");

    let mut deps = HashMap::new();
    if !input.is_dir() {
        return Ok(deps);
    }
    let dirs = WalkDir::new("INPUT")
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir());

    // TODO: run in parallel
    for d in dirs {
        let pth = d.path().strip_prefix("INPUT").unwrap();
        let component = pth.to_str().unwrap();
        let lck = try!(read_partial_lockfile(&component));
        deps.insert(component.to_string(), lck.version);
    }
    Ok(deps)
}

pub struct InputDependency {
    pub name: String,
    pub missing: bool,
    pub extraneous: bool,
    pub development: bool,
    pub version: String, // on disk
    pub requirement: Option<String>, // from manifest
}

pub type InputMap = HashMap<String, InputDependency>;


pub fn analyze_full(manifest: &Manifest) -> LalResult<InputMap> {
    let input = Path::new("./INPUT");

    let deps = try!(analyze());
    let saved_deps = manifest.all_dependencies();

    let mut depmap = InputMap::new();
    if !input.is_dir() {
        return Ok(depmap);
    }

    // check manifested deps
    for (d, v) in saved_deps.clone() {
        let actual_ver = deps.get(&d).unwrap().clone();
        depmap.insert(d.clone(),
                      InputDependency {
                          name: d.clone(),
                          version: actual_ver,
                          requirement: Some(format!("{}", v)),
                          missing: deps.get(&d).is_none(),
                          development: manifest.devDependencies.contains_key(&d),
                          extraneous: false,
                      });
    }
    // check for potentially non-manifested deps
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
