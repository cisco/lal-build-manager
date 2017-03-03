use core::LalResult;
use std::fs::File;
use semver::Version;

// TODO: no.
use super::ArtifactoryConfig;

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

    // TODO: generic config
    /// Raw config information when all encapsulation fails
    fn get_config(&self) -> ArtifactoryConfig;

    /// Publish a file into a specific location
    fn upload_file(&self, uri: &str, f: &mut File) -> LalResult<()>;

    /// How to perform an upgrade check
    fn get_latest_lal_version(&self) -> LalResult<Version>;
}

/// Behaviour we expect to have for our caching layer
pub trait Cacheable {
    /// Return the base directory to be used to dump cached downloads
    fn get_cache_dir(&self) -> String;
}
