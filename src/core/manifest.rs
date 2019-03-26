use serde_json;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::vec::Vec;

use super::{CliError, LalResult};
use crate::channel::Channel;
use crate::verify;

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
    fn default() -> Self {
        Self {
            configurations: vec!["release".to_string()],
            defaultConfig: "release".to_string(),
        }
    }
}

/// Coordinates when a channel and version are present.
#[derive(Serialize, Deserialize, Clone)]
pub struct TwoDCoordinates {
    /// Channel
    pub channel: String,
    /// Version
    pub version: u32,
}

impl std::fmt::Display for TwoDCoordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}/{}", self.channel, self.version)
    }
}

/// Coordinates enum, maps to the various ways a build's coordinates can be described.
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Coordinates {
    /// Coordinates with only a version
    OneD(u32),
    /// Coordinates with a version and a channel
    TwoD(TwoDCoordinates),
}

impl std::fmt::Display for Coordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Coordinates::OneD(c) => write!(f, "{}", c),
            Coordinates::TwoD(c) => write!(f, "{}", c),
        }
    }
}

/// Type alias used for manifest dependencies.
type Dep = BTreeMap<String, Coordinates>;

/// Representation of `manifest.json`
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Manifest {
    /// Name of the main component
    pub name: String,
    /// Channel the component is currently in
    pub channel: Option<String>,
    /// Default environment to build in
    pub environment: String,
    /// All the environments dependencies can currently be found in
    pub supportedEnvironments: Vec<String>,
    /// Components and their available configurations that are buildable
    pub components: BTreeMap<String, ComponentConfiguration>,
    /// Dependencies that are always needed
    pub dependencies: Dep,
    /// Development dependencies
    pub devDependencies: Dep,

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
    fn default() -> Self { ManifestLocation::LalSubfolder }
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
    pub fn identify(pwd: &PathBuf) -> LalResult<Self> {
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
    pub fn new(name: &str, env: &str, location: &PathBuf) -> Self {
        let mut comps = BTreeMap::new();
        comps.insert(name.into(), ComponentConfiguration::default());
        Self {
            name: name.into(),
            components: comps,
            environment: env.into(),
            supportedEnvironments: vec![env.into()],
            location: location.to_string_lossy().into(),
            ..Self::default()
        }
    }
    /// Merge dependencies and devDependencies into one convenience map
    pub fn all_dependencies(&self) -> Dep {
        let mut deps = self.dependencies.clone();
        for (k, v) in &self.devDependencies {
            deps.insert(k.clone(), v.clone());
        }
        deps
    }
    /// Read a manifest file in PWD
    pub fn read() -> LalResult<Self> { Ok(Self::read_from(&Path::new(".").to_path_buf())?) }

    /// Read a manifest file in an arbitrary path
    pub fn read_from(pwd: &PathBuf) -> LalResult<Self> {
        let mpath = ManifestLocation::identify(pwd)?.as_path(pwd);
        trace!("Using manifest in {}", mpath.display());
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mut res: Self = serde_json::from_str(&data)?;
        // store the location internally (not serialized to disk)
        res.location = mpath.to_string_lossy().into();
        Ok(res)
    }

    /// Update the manifest file in the current folder
    pub fn write(&self) -> LalResult<()> {
        let encoded = serde_json::to_string_pretty(self)?;
        trace!("Writing manifest in {}", self.location);
        let mut f = File::create(&self.location)?;
        writeln!(f, "{}", encoded)?;
        debug!("Wrote manifest in {}: \n{}", self.location, encoded);
        Ok(())
    }

    /// Verify assumptions about configurations
    pub fn verify(&self, flags: verify::Flags) -> LalResult<()> {
        for (name, conf) in &self.components {
            if &name.to_lowercase() != name {
                return Err(CliError::InvalidComponentName(name.clone()));
            }
            // Verify ComponentSettings (manifest.components[x])
            debug!("Verifying component {}", name);
            if !conf.configurations.contains(&conf.defaultConfig) {
                let ename = format!(
                    "default configuration '{}' not found in configurations list",
                    conf.defaultConfig
                );
                return Err(CliError::InvalidBuildConfiguration(ename));
            }
        }
        for (name, values) in self.dependencies.iter().chain(self.devDependencies.iter()) {
            if &name.to_lowercase() != name {
                return Err(CliError::InvalidComponentName(name.clone()));
            }

            match values {
                Coordinates::OneD(_) => (),
                Coordinates::TwoD(v) => {
                    let child = Channel::from_option(&self.channel);
                    let parent = Channel::new(&v.channel);
                    child.contains(&parent)?;
                }
            }
        }
        if self.supportedEnvironments.is_empty() {
            return Err(CliError::NoSupportedEnvironments);
        }
        if !self
            .supportedEnvironments
            .iter()
            .any(|x| x == &self.environment)
        {
            return Err(CliError::UnsupportedEnvironment);
        }
        if let Some(ch) = &self.channel {
            let channel = Channel::new(ch);
            if channel.to_string() != *ch {
                return Err(CliError::InvalidChannelName(ch.to_string()));
            }

            let allow_testing = flags.contains(verify::Flags::TESTING);
            if !allow_testing && channel.is_testing() {
                return Err(CliError::InvalidTestingChannel(ch.to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_ok {
        ( $e:expr ) => {
            match $e {
                Ok(_) => (),
                Err(e) => {
                    println!("{:?}", e);
                    assert!(false);
                }
            }
        };
    }

    macro_rules! assert_no {
        ( $e:expr ) => {
            match $e {
                Ok(_) => assert!(false),
                Err(_) => (),
            }
        };
    }

    #[test]
    fn test_verify() {
        let mut manifest = Manifest::new("", "", &PathBuf::default());
        assert_ok!(manifest.verify(verify::Flags::default()));
        assert_ok!(manifest.verify(verify::Flags::SIMPLE));
        assert_ok!(manifest.verify(verify::Flags::TESTING));
        assert_ok!(manifest.verify(verify::Flags::TESTING | verify::Flags::SIMPLE));

        manifest.channel = Some("/".to_string());
        assert_ok!(manifest.verify(verify::Flags::default()));
        assert_ok!(manifest.verify(verify::Flags::SIMPLE));
        assert_ok!(manifest.verify(verify::Flags::TESTING));
        assert_ok!(manifest.verify(verify::Flags::TESTING | verify::Flags::SIMPLE));

        manifest.channel = Some("/testing".to_string());
        assert_no!(manifest.verify(verify::Flags::default()));
        assert_no!(manifest.verify(verify::Flags::SIMPLE));
        assert_ok!(manifest.verify(verify::Flags::TESTING));
        assert_ok!(manifest.verify(verify::Flags::TESTING | verify::Flags::SIMPLE));

        manifest.channel = Some("/a/testing".to_string());
        assert_no!(manifest.verify(verify::Flags::default()));
        assert_no!(manifest.verify(verify::Flags::SIMPLE));
        assert_ok!(manifest.verify(verify::Flags::TESTING));
        assert_ok!(manifest.verify(verify::Flags::TESTING | verify::Flags::SIMPLE));

        manifest.channel = Some("".to_string());
        assert_no!(manifest.verify(verify::Flags::default()));
        assert_no!(manifest.verify(verify::Flags::SIMPLE));
        assert_no!(manifest.verify(verify::Flags::TESTING));
        assert_no!(manifest.verify(verify::Flags::TESTING | verify::Flags::SIMPLE));
    }
}
