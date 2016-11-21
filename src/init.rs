use std::io::prelude::*;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::collections::BTreeMap;
use std::vec::Vec;
use rustc_serialize::json;

use super::{Config, CliError, LalResult};

/// Representation of a value of the manifest.components hash
#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct ComponentConfiguration {
    /// The default config to use if not passed in - default is "release"
    pub defaultConfig: String,
    /// List of allowed configurations (must contain defaultConfig)
    pub configurations: Vec<String>,
}

impl Default for ComponentConfiguration {
    fn default() -> ComponentConfiguration {
        ComponentConfiguration {
            configurations: vec!["release".to_string()],
            defaultConfig: "release".to_string(),
        }
    }
}

/// Representation of `manifest.json`
#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone, Default)]
pub struct Manifest {
    /// Name of the main component
    pub name: String,
    /// Default environment to build in
    pub environment: Option<String>,
    /// Components and their available configurations that are buildable
    pub components: BTreeMap<String, ComponentConfiguration>,
    /// Dependencies that are always needed
    pub dependencies: BTreeMap<String, u32>,
    /// Development dependencies
    pub devDependencies: BTreeMap<String, u32>,
}

impl Manifest {
    /// Initialize a manifest struct based on a name
    ///
    /// The name is assumed to be the default component and will create a
    /// component configuration for it with its default values.
    pub fn new(name: &str, env: &str) -> Manifest {
        let mut comps = BTreeMap::new();
        comps.insert(name.into(), ComponentConfiguration::default());
        Manifest {
            name: name.into(),
            components: comps,
            environment: Some(env.into()),
            ..Default::default()
        }
    }
    /// Merge dependencies and devDependencies into one convenience map
    pub fn all_dependencies(&self) -> BTreeMap<String, u32> {
        let mut deps = self.dependencies.clone();
        for (k, v) in &self.devDependencies {
            deps.insert(k.clone(), *v);
        }
        deps
    }
    /// Read a manifest file in PWD
    pub fn read() -> LalResult<Manifest> {
        Ok(Manifest::read_from(Path::new(".").to_path_buf())?)
    }
    /// Read a manifest file in an arbitrary path
    pub fn read_from(pth: PathBuf) -> LalResult<Manifest> {
        let mpath = pth.join("manifest.json");
        if !mpath.exists() {
            return Err(CliError::MissingManifest);
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let res = json::decode(&data)?;
        Ok(res)
    }

    /// Update the manifest file in the current folder
    pub fn write(&self) -> LalResult<()> {
        let pth = Path::new("./manifest.json");
        let encoded = json::as_pretty_json(self);

        let mut f = File::create(&pth)?;
        write!(f, "{}\n", encoded)?;

        info!("Wrote manifest {}: \n{}", pth.display(), encoded);
        Ok(())
    }
}

/// Helper to print the dependencies from the manifest
pub fn dep_list(mf: &Manifest, core: bool) -> LalResult<()> {
    let deps = if core { mf.dependencies.clone() } else { mf.all_dependencies() };
    for k in deps.keys() {
        println!("{}", k);
    }
    Ok(())
}

/// Generates a blank manifest in the current directory
///
/// This will use the directory name as the assumed default component name
/// Then fill in the blanks as best as possible.
///
/// The function will not overwrite an existing `manifest.json`,
/// unless the `force` bool is set.
pub fn init(cfg: &Config, force: bool, env: &str) -> LalResult<()> {
    cfg.get_container(Some(env.into()))?;

    let pwd = env::current_dir()?;
    let last_comp = pwd.components().last().unwrap(); // std::path::Component
    let dirname = last_comp.as_os_str().to_str().unwrap();

    let manifest_path = Path::new("manifest.json");
    if !force && manifest_path.exists() {
        return Err(CliError::ManifestExists);
    }

    Manifest::new(dirname, env).write()?;
    Ok(())
}
