use rustc_serialize::json;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::env;

use std::collections::HashMap;
use init::Manifest;
use errors::LalResult;


// TODO: need a struct
// TODO: try to parse a versions.yaml

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: u32,
    // TODO: other stash data if using a stashed build
    pub dependencies: HashMap<String, Dependency>
}

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Lock {
    pub name: String,
    //pub date: String,
    pub version: u32,
    pub dependencies: HashMap<String, Dependency>,
}


// The main interface from build()
pub fn generate(m: &Manifest) -> LalResult<()> {
    let lock = Lock {
        name: m.name.clone(),
        version: m.version,
        dependencies: HashMap::new(),
    };
    let encoded = json::as_pretty_json(&lock);

    let pwd = env::current_dir().unwrap();
    let lockfile = Path::new(&pwd).join("ARTIFACT").join("lockfile.json");
    let mut f = try!(File::create(&lockfile));
    try!(write!(f, "{}\n", encoded));

    info!("Wrote lockfile {}: \n{}", lockfile.display(), encoded);
    Ok(())
}
