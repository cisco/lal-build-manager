use std::path::PathBuf;

use core::LalResult;
use super::{ArtifactoryConfig, LocalConfig};

/// An enum struct for the currently configured `Backend`
///
/// Any other implementations should be imported and listed here.
/// Currently only artifactory is supported.
#[derive(Serialize, Deserialize, Clone)]
pub enum BackendConfiguration {
    /// Config for the `ArtifactoryBackend`
    #[serde(rename = "artifactory")]
    Artifactory(ArtifactoryConfig),

    /// Config for the `LocalBackend`
    #[serde(rename = "local")]
    Local(LocalConfig),
}

/// Artifactory is the default backend
impl Default for BackendConfiguration {
    fn default() -> Self { BackendConfiguration::Artifactory(ArtifactoryConfig::default()) }
}


/// The basic definition of a component as it exists online
///
/// A component may have many build artifacts from many environments.
pub struct Component {
    /// Name of the component
    pub name: String,
    /// Version number
    pub version: u32,
    /// The raw location of the component at the specified version number
    ///
    /// No restriction on how this information is encoded, but it must work with `raw_fetch`
    pub location: String,
}

/// Properties a storage backend of artifacts should have
///
/// We are not really relying on Artifactory specific quirks in our default usage
/// so that in case it fails it can be switched over.
/// We do rely on there being a basic API that can implement this trait though.
pub trait Backend {
    /// Get a list of versions for a component in descending order
    fn get_versions(&self, name: &str, loc: &str) -> LalResult<Vec<u32>>;
    /// Get the latest version of a component
    fn get_latest_version(&self, name: &str, loc: &str) -> LalResult<u32>;

    /// Get the version and location information of a component
    ///
    /// If no version is given, figure out what latest is
    fn get_component_info(&self, name: &str, ver: Option<u32>, loc: &str) -> LalResult<Component>;

    /// Publish a release build's ARTIFACT to a specific location
    ///
    /// This will publish everything inside the ARTIFACT dir created by `lal build -r`
    fn publish_artifact(&self, name: &str, version: u32, env: &str) -> LalResult<()>;

    /// Raw fetch of location to a destination
    ///
    /// location can be a HTTPS url / a system path / etc (depending on the backend)
    fn raw_fetch(&self, location: &str, dest: &PathBuf) -> LalResult<()>;

    /// Return the base directory to be used to dump cached downloads
    ///
    /// This has to be in here for `CachedBackend` to have a straight dependency
    fn get_cache_dir(&self) -> String;
}

/// A secondary trait that builds upon the Backend trait
///
/// This wraps the common fetch commands in a caching layer on the cache dir.
pub trait CachedBackend {
    /// Get the latest version of a component across all supported environments
    fn get_latest_supported_versions(
        &self,
        name: &str,
        environments: Vec<String>,
    ) -> LalResult<Vec<u32>>;

    /// Retrieve the location to a cached published component (downloading if necessary)
    fn retrieve_published_component(
        &self,
        name: &str,
        version: Option<u32>,
        env: &str,
    ) -> LalResult<(PathBuf, Component)>;

    /// Retrieve the location to a stashed component
    fn retrieve_stashed_component(&self, name: &str, code: &str) -> LalResult<PathBuf>;

    /// Retrieve and unpack a cached component in INPUT
    fn unpack_published_component(
        &self,
        name: &str,
        version: Option<u32>,
        env: &str,
    ) -> LalResult<Component>;

    /// Retrieve and unpack a stashed component to INPUT
    fn unpack_stashed_component(&self, name: &str, code: &str) -> LalResult<()>;

    /// Add a stashed component from a folder
    fn stash_output(&self, name: &str, code: &str) -> LalResult<()>;
}
