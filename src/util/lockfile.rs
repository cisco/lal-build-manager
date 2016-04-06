use rustc_serialize::json;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;

use std::collections::HashMap;

use errors::{CliError, LalResult};
use util::input;
use init::Manifest;

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Lock {
    pub name: String,
    pub config: String,
    // pub date: String,
    pub version: String,
    pub dependencies: HashMap<String, Lock>,
    // TODO: container to avoid abi smashing
    // TODO: other stash data if using a stashed build?
}

impl Lock {
    pub fn new(n: &str, v: Option<&str>, build_cfg: &str) -> Lock {
        Lock {
            name: n.to_string(),
            version: v.unwrap_or("experimental").to_string(),
            config: build_cfg.to_string(),
            dependencies: HashMap::new(),
        }
    }
    pub fn populate_from_input(mut self, manifest: &Manifest) -> LalResult<Self> {
        let deps = try!(input::analyze_full(manifest));
        for (name, dep) in deps {
            info!("got dep {} {}", name, dep.version);
            let deplock = try!(read_lockfile_from_component(&name));
            self.dependencies.insert(name.clone(), deplock);
        }
        Ok(self)
    }
    pub fn write(&self, pth: &Path) -> LalResult<()> {
        let encoded = json::as_pretty_json(self);
        let mut f = try!(File::create(pth));
        try!(write!(f, "{}\n", encoded));
        info!("Wrote lockfile {}: \n{}", pth.display(), encoded);
        Ok(())
    }

    pub fn validate(&self) -> bool {
        unimplemented!()
    }
}

fn read_lockfile_from_component(component: &str) -> LalResult<Lock> {
    let lock_path = Path::new("./INPUT").join(component).join("lockfile.json");
    if ! lock_path.exists() {
        return Err(CliError::MissingLockfile(component.to_string()));
    }
    let mut lock_str = String::new();
    try!(try!(File::open(&lock_path)).read_to_string(&mut lock_str));
    let res = try!(json::decode(&lock_str));
    Ok(res)
}
