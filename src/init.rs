use std::io::prelude::*;
use std::env;
use std::path::Path;
use std::fs::File;
use std::collections::BTreeMap;
use std::vec::Vec;
use rustc_serialize::json;

use errors::{CliError, LalResult};

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct ComponentConfiguration {
    pub defaultConfig: String,
    pub configurations: Vec<String>,
}
impl ComponentConfiguration {
    pub fn new() -> ComponentConfiguration {
        ComponentConfiguration {
            configurations: vec!["release".to_string()],
            defaultConfig: "release".to_string(),
        }
    }
}

/// Representation of `manifest.json`
#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Manifest {
    pub name: String,
    pub components: BTreeMap<String, ComponentConfiguration>,
    pub dependencies: BTreeMap<String, u32>,
    pub devDependencies: BTreeMap<String, u32>,
}

impl Manifest {
    pub fn new(n: &str) -> Manifest {
        let mut comps = BTreeMap::new();
        comps.insert(n.to_string(), ComponentConfiguration::new());
        Manifest {
            name: n.to_string(),
            components: comps,
            dependencies: BTreeMap::new(),
            devDependencies: BTreeMap::new(),
        }
    }
    pub fn all_dependencies(&self) -> BTreeMap<String, u32> {
        let mut deps = self.dependencies.clone();
        for (k, v) in &self.devDependencies {
            deps.insert(k.clone(), v.clone());
        }
        deps
    }
    pub fn read() -> LalResult<Manifest> {
        let manifest_path = Path::new("./manifest.json");
        if !manifest_path.exists() {
            return Err(CliError::MissingManifest);
        }
        let mut f = try!(File::open(&manifest_path));
        let mut manifest_str = String::new();
        try!(f.read_to_string(&mut manifest_str));
        let res = try!(json::decode(&manifest_str));
        Ok(res)
    }

    pub fn write(&self) -> LalResult<()> {
        let pth = Path::new("./manifest.json");
        let encoded = json::as_pretty_json(self);

        let mut f = try!(File::create(&pth));
        try!(write!(f, "{}\n", encoded));

        info!("Wrote manifest {}: \n{}", pth.display(), encoded);
        Ok(())
    }
}

/// Generates a blank manifest in the current directory
///
/// This will use the directory name as the assumed default component name
/// Then fill in the blanks as best as possible.
///
/// The function will not overwrite an existing `manifest.json`,
/// unless the `force` bool is set.
pub fn init(force: bool) -> LalResult<()> {
    let pwd = try!(env::current_dir());
    let last_comp = pwd.components().last().unwrap(); // std::path::Component
    let dirname = last_comp.as_os_str().to_str().unwrap();

    let manifest_path = Path::new("./manifest.json");
    if !force && manifest_path.exists() {
        return Err(CliError::ManifestExists);
    }

    try!(Manifest::new(dirname).write());
    Ok(())
}
