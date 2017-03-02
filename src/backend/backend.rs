use core::LalResult;
use std::fs::File;

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
    /// Bucket the tarball was found in
    pub location: Option<String>,
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

    // TODO: another set of two for Semver components?

    /// Get the tarball url of a `Component` in a backend location
    /// If no version is given, return latest
    fn get_tarball_url(&self,
                       name: &str,
                       version: Option<u32>,
                       loc: Option<&str>)
                       -> LalResult<Component>;

    /// Publish a file into a specific location
    fn upload_file(&self, uri: String, f: &mut File) -> LalResult<()>;
}
