use std::io::prelude::*;
use std::env;
use std::path::Path;
use std::fs::File;
use std::collections::HashMap;
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

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Manifest {
    pub name: String,
    pub components: HashMap<String, ComponentConfiguration>,
    pub dependencies: HashMap<String, u32>,
    pub devDependencies: HashMap<String, u32>,
}

impl Manifest {
    pub fn new(n: &str) -> Manifest {
        let mut comps = HashMap::new();
        comps.insert(n.to_string(), ComponentConfiguration::new());
        Manifest {
            name: n.to_string(),
            components: comps,
            dependencies: HashMap::new(),
            devDependencies: HashMap::new(),
        }
    }
    pub fn all_dependencies(&self) -> HashMap<String, u32> {
        let mut deps = self.dependencies.clone();
        for (k, v) in &self.devDependencies {
            deps.insert(k.clone(), v.clone());
        }
        deps
    }
    pub fn read() -> LalResult<Manifest> {
        let pwd = try!(env::current_dir());
        let manifest_path = Path::new(&pwd).join("manifest.json");
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
        let pwd = try!(env::current_dir());;
        let pth = Path::new(&pwd).join("manifest.json");
        let encoded = json::as_pretty_json(self);

        let mut f = try!(File::create(&pth));
        try!(write!(f, "{}\n", encoded));

        info!("Wrote manifest {}: \n{}", pth.display(), encoded);
        Ok(())
    }
}

pub fn init(force: bool) -> LalResult<()> {
    let pwd = try!(env::current_dir());
    let last_comp = pwd.components().last().unwrap(); // std::path::Component
    let dirname = last_comp.as_os_str().to_str().unwrap();

    let manifest_path = Path::new(&pwd).join("manifest.json");
    if !force && manifest_path.exists() {
        return Err(CliError::ManifestExists);
    }

    try!(Manifest::new(dirname).write());
    Ok(())
}
