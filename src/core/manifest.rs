use std::io::prelude::*;
use std::fs::{self, File};
use std::collections::BTreeMap;
use std::vec::Vec;
use serde_json;
use std::path::{Path, PathBuf};

use super::{CliError, LalResult};

/// A startup helper used in a few places
pub fn create_lal_subdir(pwd: &PathBuf) -> LalResult<()> {
    let loc = pwd.join(".lal");
    if !loc.is_dir() {
        fs::create_dir(&loc)?
    }
    Ok(())
}

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
    pub environment: String,
    /// All the environments dependencies can currently be found in
    pub supportedEnvironments: Vec<String>,
    /// Components and their available configurations that are buildable
    pub components: BTreeMap<String, ComponentConfiguration>,
    /// Dependencies that are always needed
    pub dependencies: BTreeMap<String, u32>,
    /// Development dependencies
    pub devDependencies: BTreeMap<String, u32>,

    /// Internal path of this manifest
    #[serde(skip_serializing, skip_deserializing)]
    location: String,
}

/// An enum to clarify intent
pub enum ManifestLocation {
    /// Plain style (old default)
    RepoRoot,
    /// In the .lal subfolder
    LalSubfolder,
}
impl Default for ManifestLocation {
    fn default() -> ManifestLocation { ManifestLocation::LalSubfolder }
}
impl ManifestLocation {
    /// Generate path for Manifest assuming pwd is the root
    pub fn as_path(&self, pwd: &PathBuf) -> PathBuf {
        match *self {
            ManifestLocation::RepoRoot => pwd.join("manifest.json"),
            ManifestLocation::LalSubfolder => pwd.join(".lal/manifest.json"),
        }
    }

    /// Find the manifest file
    ///
    /// Looks first in `./.lal/manifest.json` and falls back to `./manifest.json`
    pub fn identify(pwd: &PathBuf) -> LalResult<ManifestLocation> {
        if ManifestLocation::LalSubfolder.as_path(pwd).exists() {
            // Show a warning if we have two manifests - we only use the new one then
            // This could happen on other codebases - some javascript repos use manifest.json
            // if both are for lal though, then this is user error, make it explicit:
            if ManifestLocation::RepoRoot.as_path(pwd).exists() {
                warn!("manifest.json found in both .lal/ and current directory");
                warn!("Using the default: .lal/manifest.json");
            }
            Ok(ManifestLocation::LalSubfolder)
        } else if ManifestLocation::RepoRoot.as_path(pwd).exists() {
            Ok(ManifestLocation::RepoRoot) // allow people to migrate for a while
        } else {
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
            environment: env.into(),
            supportedEnvironments: vec![env.into()],
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
    pub fn read() -> LalResult<Manifest> { Ok(Manifest::read_from(&Path::new(".").to_path_buf())?) }

    /// Read a manifest file in an arbitrary path
    pub fn read_from(pwd: &PathBuf) -> LalResult<Manifest> {
        let mpath = ManifestLocation::identify(pwd)?.as_path(pwd);
        trace!("Using manifest in {}", mpath.display());
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mut res: Manifest = serde_json::from_str(&data)?;
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
        debug!("Wrote manifest in {}: \n{}", self.location, encoded);
        Ok(())
    }

    /// Verify assumptions about configurations
    pub fn verify(&self) -> LalResult<()> {
        for (name, conf) in &self.components {
            if &name.to_lowercase() != name {
                return Err(CliError::InvalidComponentName(name.clone()));
            }
            // Verify ComponentSettings (manifest.components[x])
            debug!("Verifying component {}", name);
            if !conf.configurations.contains(&conf.defaultConfig) {
                let ename = format!("default configuration '{}' not found in configurations list",
                                    conf.defaultConfig);
                return Err(CliError::InvalidBuildConfiguration(ename));
            }
        }
        for (name, _) in &self.dependencies {
            if &name.to_lowercase() != name {
                return Err(CliError::InvalidComponentName(name.clone()));
            }
        }
        for (name, _) in &self.devDependencies {
            if &name.to_lowercase() != name {
                return Err(CliError::InvalidComponentName(name.clone()));
            }
        }
        if self.supportedEnvironments.is_empty() {
            return Err(CliError::NoSupportedEnvironments);
        }
        if !self.supportedEnvironments.iter().any(|x| x == &self.environment) {
            return Err(CliError::UnsupportedEnvironment);
        }
        Ok(())
    }
}
