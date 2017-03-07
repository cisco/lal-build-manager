use std::fs::File;
use std::path::PathBuf;
use semver::Version;

use core::LalResult;
use super::ArtifactoryConfig;

/// An enum struct for the currently configured `Backend`
///
/// Any other implementations should be imported and listed here.
/// Currently only artifactory is supported.
#[derive(Serialize, Deserialize, Clone)]
pub enum BackendConfiguration {
    /// Config for the `ArtifactoryBackend`
    #[serde(rename = "artifactory")]
    Artifactory(ArtifactoryConfig),
}

/// Artifactory is the default backend
impl Default for BackendConfiguration {
    fn default() -> Self {
        BackendConfiguration::Artifactory(ArtifactoryConfig::default())
    }
}


/// The basic definition of a component as it exists online
///
/// A component may have many build artifacts from many environments.
pub struct Component {
    /// Name of the component
    pub name: String,
    /// Version number
    pub version: u32,
    /// The raw URL of the tarball at the specified version number
    pub tarball: String,
}

/// Properties a storage backend of artifacts should have
///
/// We are not really relying on Artifactory specific quirks in our default usage
/// so that in case it fails it can be switched over.
/// We do rely on there being a basic API that can implement this trait though.
pub trait Backend {
    /// Get a list of versions for a component
    fn get_versions(&self, name: &str, loc: Option<&str>) -> LalResult<Vec<u32>>;
    /// Get the latest version of a component
    fn get_latest_version(&self, name: &str, loc: Option<&str>) -> LalResult<u32>;

    /// Get the tarball url of a `Component` in a backend location
    /// If no version is given, return latest
    fn get_tarball_url(&self,
                       name: &str,
                       version: Option<u32>,
                       loc: Option<&str>)
                       -> LalResult<Component>;

    /// Publish a file into a specific location
    fn upload_file(&self, uri: &str, f: &mut File) -> LalResult<()>;

    /// How to perform an upgrade check
    fn get_latest_lal_version(&self) -> LalResult<Version>;
    /// Where to fetch latest upgrade tarball
    fn get_lal_upgrade_url(&self) -> String;

    /// Raw dowlnload of a url to a destination
    fn raw_download(&self, url: &str, dest: &PathBuf) -> LalResult<()>;

    /// Return the base directory to be used to dump cached downloads
    /// This has to be in here for `CachedBackend` to have a straight dependency
    fn get_cache_dir(&self) -> String;
}

/// A secondary trait that builds upon the Backend trait
/// This wraps the common fetch commands in a caching layer on the cache dir.
pub trait CachedBackend {
    /// Retrieve the location to a cached published component (downloading if necessary)
    fn retrieve_published_component(&self,
                                    name: &str,
                                    version: Option<u32>,
                                    env: Option<&str>)
                                    -> LalResult<(PathBuf, Component)>;

    /// Retrieve the location to a stashed component
    fn retrieve_stashed_component(&self, name: &str, code: &str) -> LalResult<PathBuf>;

    /// Retrieve and unpack a cached component in INPUT
    fn unpack_published_component(&self,
                                  name: &str,
                                  version: Option<u32>,
                                  env: Option<&str>)
                                  -> LalResult<Component>;

    /// Retrieve and unpack a stashed component to INPUT
    fn unpack_stashed_component(&self, name: &str, code: &str) -> LalResult<()>;

    /// Add a stashed component from a folder
    fn stash_output(&self, name: &str, code: &str) -> LalResult<()>;
}
