use std::io::prelude::*;
use std::process;
use std::env;
use std::path::Path;
use std::fs::File;
use std::collections::HashMap;
use rustc_serialize::json;

use errors::CliError;

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub dependencies: HashMap<String, u32>,
    pub devDependencies: HashMap<String, u32>,
}

pub fn read_manifest() -> Result<Manifest, CliError> {
    let pwd = env::current_dir().unwrap();
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

pub fn save_manifest(m: &Manifest) -> Result<(), CliError> {
    let pwd = env::current_dir().unwrap();
    let encoded = json::as_pretty_json(&m);

    let manifest_path = Path::new(&pwd).join("manifest.json");
    let mut f = try!(File::create(&manifest_path));
    try!(write!(f, "{}\n", encoded));
    info!("Wrote manifest {}: \n{}", manifest_path.display(), encoded);
    Ok(())
}

pub fn init(force: bool) -> Result<Manifest, CliError> {
    let pwd = env::current_dir().unwrap();
    let last_comp = pwd.components().last().unwrap(); // std::path::Component
    let dirname = last_comp.as_os_str().to_str().unwrap();

    let manifest = Manifest {
        name: dirname.to_string(),
        version: "0".to_string(),
        dependencies: HashMap::new(),
        devDependencies: HashMap::new(),
    };

    let encoded = json::as_pretty_json(&manifest);

    let manifest_path = Path::new(&pwd).join("manifest.json");
    if !force && manifest_path.exists() {
        println!("manifest.json already exists, stopping.");
        println!("Use -f to overwrite");
        process::exit(1);
    }
    let mut f = try!(File::create(&manifest_path));
    try!(write!(f, "{}\n", encoded));

    info!("Wrote manifest {}: \n{}", manifest_path.display(), encoded);
    Ok(manifest.clone())
}
