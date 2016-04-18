use rustc_serialize::json;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;

use std::collections::HashMap;
use std::collections::BTreeSet;

use errors::{CliError, LalResult};
use util::input;

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Container {
    name: String,
    tag: String,
}

/// Representation of `lockfile.json`
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Lockfile {
    pub name: String,
    pub config: String,
    pub container: Container,
    // pub date: String,
    pub version: String,
    pub tool: String,
    pub dependencies: HashMap<String, Lockfile>,
}

impl Lockfile {
    pub fn new(n: &str, container: &str, v: Option<&str>, build_cfg: Option<&str>) -> Lockfile {
        let split : Vec<&str> = container.split(":").collect();
        let tag = if split.len() == 2 { split[1] } else { "latest" };
        let cname = if split.len() == 2 { split[0] } else { container };
        Lockfile {
            name: n.to_string(),
            version: v.unwrap_or("EXPERIMENTAL").to_string(),
            config: build_cfg.unwrap_or("release").to_string(),
            container: Container {
                name: cname.to_string(),
                tag: tag.to_string(),
            },
            tool: env!("CARGO_PKG_VERSION").to_string(),
            dependencies: HashMap::new(),
        }
    }
    pub fn populate_from_input(mut self) -> LalResult<Self> {
        // NB: this is not a particularly smart algorithm
        // We read all the lockfiles easily in analyze
        // Then we re-read them fully in read_lockfile_from_component
        let deps = try!(input::analyze());
        for (name, _) in deps {
            trace!("Populating lockfile with {}", name);
            let deplock = try!(read_lockfile_from_component(&name));
            self.dependencies.insert(name.clone(), deplock);
        }
        Ok(self)
    }
    pub fn write(&self, pth: &Path, silent: bool) -> LalResult<()> {
        let encoded = json::as_pretty_json(self);
        let mut f = try!(File::create(pth));
        try!(write!(f, "{}\n", encoded));
        if silent {
            debug!("Wrote lockfile {}: \n{}", pth.display(), encoded);
        } else {
            info!("Wrote lockfile {}: \n{}", pth.display(), encoded);
        }
        Ok(())
    }
}

// name of component -> (ver, other_ver, ..)
pub type DependencyUsage = HashMap<String, BTreeSet<String>>;
pub fn find_all_dependencies(lock: &Lockfile) -> DependencyUsage {
    let mut acc = HashMap::new();
    // for each entry in dependencies
    for (main_name, dep) in &lock.dependencies {
        // Store the dependency
        if !acc.contains_key(main_name) {
            acc.insert(main_name.clone(), BTreeSet::new());
        }
        {
            // Only borrow as mutable once - so creating a temporary scope
            let first_version_set = acc.get_mut(main_name).unwrap();
            first_version_set.insert(dep.version.clone());
        }

        // Recurse into its dependencies
        trace!("Recursing into deps for {}, acc is {:?}", main_name, acc);
        for (name, version_set) in find_all_dependencies(&dep) {
            trace!("Found versions for for {} under {} as {:?}",
                   name,
                   main_name,
                   version_set);
            // ensure each entry from above exists in current accumulator
            if !acc.contains_key(&name) {
                acc.insert(name.clone(), BTreeSet::new());
            }
            // union the entry of versions for the current name
            let full_version_set = acc.get_mut(&name).unwrap(); // know this exists now
            for version in version_set {
                full_version_set.insert(version);
            }
        }
    }
    acc
}


fn read_lockfile_from_component(component: &str) -> LalResult<Lockfile> {
    let lock_path = Path::new("./INPUT").join(component).join("lockfile.json");
    if !lock_path.exists() {
        return Err(CliError::MissingLockfile(component.to_string()));
    }
    let mut lock_str = String::new();
    try!(try!(File::open(&lock_path)).read_to_string(&mut lock_str));
    let res = try!(json::decode(&lock_str));
    Ok(res)
}
