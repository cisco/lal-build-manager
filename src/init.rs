use std::io::prelude::*;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::{File, self};
use std::collections::BTreeMap;
use std::vec::Vec;
use serde_json;

use super::{Config, CliError, LalResult};

/// Representation of a value of the manifest.components hash
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone)]
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
#[derive(Serialize, Deserialize, Clone, Default)]
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

    /// Internal path of this manifest
    #[serde(skip_serializing,skip_deserializing)]
    location: String,
}

/// An enum to clarify intent
pub enum ManifestLocation {
    // plain style (old default)
    RepoRoot,
    // hidden
    LalSubfolder,
}
impl Default for ManifestLocation {
    fn default() -> ManifestLocation {
        ManifestLocation::LalSubfolder
    }
}
impl ManifestLocation {
    fn as_path(&self, pwd: &PathBuf) -> PathBuf {
        match *self {
            ManifestLocation::RepoRoot => pwd.join("manifest.json"),
            ManifestLocation::LalSubfolder =>  pwd.join(".lal/manifest.json")
        }
    }

    /// Find the manifest file
    ///
    /// Looks first in `./.lal/manifest.json` and falls back to `./manifest.json`
    fn identify(pwd: &PathBuf) -> LalResult<ManifestLocation> {
        if ManifestLocation::LalSubfolder.as_path(&pwd).exists() {
            // Show a warning if we have two manifests - we only use the new one then
            // This could happen on other codebases - some javascript repos use manifest.json
            // if both are for lal though, then this is user error, make it explicit:
            if ManifestLocation::RepoRoot.as_path(&pwd).exists() {
                warn!("manifest.json found in both .lal/ and current directory");
                warn!("Using the default: .lal/manifest.json");
            }
            Ok(ManifestLocation::LalSubfolder)
        } else if ManifestLocation::RepoRoot.as_path(&pwd).exists() {
            Ok(ManifestLocation::RepoRoot) // allow people to migrate for a while
        }
        else {
            Err(CliError::MissingManifest)
        }
    }
}


impl Manifest {
    /// Initialize a manifest struct based on a name
    ///
    /// The name is assumed to be the default component and will create a
    /// component configuration for it with its default values.
    pub fn new(name: &str, env: &str, location: PathBuf) -> Manifest {
        let mut comps = BTreeMap::new();
        comps.insert(name.into(), ComponentConfiguration::default());
        Manifest {
            name: name.into(),
            components: comps,
            environment: Some(env.into()),
            location: location.to_string_lossy().into(),
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
        Ok(Manifest::read_from(&Path::new(".").to_path_buf())?)
    }

    /// Read a manifest file in an arbitrary path
    pub fn read_from(pwd: &PathBuf) -> LalResult<Manifest> {
        let mpath = ManifestLocation::identify(&pwd)?.as_path(&pwd);
        trace!("Using manifest in {}", mpath.display());
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mut res : Manifest = serde_json::from_str(&data)?;
        // store the location internally (not serialized to disk)
        res.location = mpath.to_string_lossy().into();
        Ok(res)
    }

    /// Update the manifest file in the current folder
    pub fn write(&self) -> LalResult<()> {
        let encoded = serde_json::to_string_pretty(self)?;
        trace!("Writing manifest in {}", self.location);
        let mut f = File::create(&self.location)?;
        write!(f, "{}\n", encoded)?;
        info!("Wrote manifest in {}: \n{}", self.location, encoded);
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


fn create_lal_subdir(pwd: &PathBuf) -> LalResult<()> {
    let loc = pwd.join(".lal");
    if !loc.is_dir() {
        fs::create_dir(&loc)?
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

    let mpath = ManifestLocation::identify(&pwd);
    if !force && mpath.is_ok() {
        return Err(CliError::ManifestExists);
    }

    // we are allowed to overwrite or write a new manifest if we are here
    // always create new manifests in new default location
    create_lal_subdir(&pwd)?; // create the `.lal` subdir if it's not there already
    Manifest::new(dirname, env, ManifestLocation::default().as_path(&pwd)).write()?;

    // if the manifest already existed, warn about this now being placed elsewhere
    if let Ok(ManifestLocation::RepoRoot) = mpath {
        warn!("Created manifest in new location under .lal");
        warn!("Please delete the old manifest - it will not be read anymore");
    }

    Ok(())
}
