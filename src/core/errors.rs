use std::fmt;
use std::io;
use hyper;
use serde_json;

/// The one and only error type for the lal library
///
/// Every command will raise one of these on failure, and these is some reuse between
/// commands for these errors. `Result<T, CliError>` is effectively the safety net
/// that every single advanced call goes through to avoid `panic!`
#[derive(Debug)]
pub enum CliError {
    /// Errors propagated from `std::fs`
    Io(io::Error),
    /// Errors propagated from `serde_json`
    Parse(serde_json::error::Error),
    /// Errors propagated from `hyper`
    Hype(hyper::Error),

    // main errors
    /// Manifest file not found in working directory
    MissingManifest,
    /// Config not found in ~/.lal
    MissingConfig,
    /// Component not found in manifest
    MissingComponent(String),
    /// Value in manifest is not lowercase
    InvalidComponentName(String),
    /// Manifest cannot be overwritten without forcing
    ManifestExists,
    /// Executable we shell out to is missing
    ExecutableMissing(String),
    /// lal version required by config is too old
    OutdatedLal(String, String),
    /// Missing SSL certificates
    MissingSslCerts,
    /// Root user encountered
    UnmappableRootUser,
    /// Missing predefined mount
    MissingMount(String),

    // status/verify errors
    /// Core dependencies missing in INPUT
    MissingDependencies,
    /// Cyclical dependency loop found in INPUT
    DependencyCycle(String),
    /// Dependency present at wrong version
    InvalidVersion(String),
    /// Extraneous dependencies in INPUT
    ExtraneousDependencies(String),
    /// No lockfile found for a component in INPUT
    MissingLockfile(String),
    /// Multiple versions of a component was involved in this build
    MultipleVersions(String),
    /// Multiple environments was used to build a component
    MultipleEnvironments(String),
    /// Environment for a component did not match our expected environment
    EnvironmentMismatch(String, String),
    /// Custom versions are stashed in INPUT which will not fly on Jenkins
    NonGlobalDependencies(String),
    /// No supported environments in the manifest
    NoSupportedEnvironments,
    /// Environment in manifest is not in the supported environments
    UnsupportedEnvironment,

    // env related errors
    /// Specified environment is not present in the main config
    MissingEnvironment(String),
    /// Command now requires an environment specified
    EnvironmentUnspecified,

    // build errors
    /// Build configurations does not match manifest or user input
    InvalidBuildConfiguration(String),
    /// BUILD script not executable
    BuildScriptNotExecutable(String),
    /// BUILD script not found
    MissingBuildScript,

    // script errors
    /// Script not found in local .lal/scripts/ directory
    MissingScript(String),

    // cache errors
    /// Failed to find a tarball after fetching from artifactory
    MissingTarball,
    /// Failed to find build artifacts in OUTPUT after a build or before stashing
    MissingBuild,

    // stash errors
    /// Invalid integer name used with lal stash
    InvalidStashName(u32),
    /// Failed to find stashed artifact in the lal cache
    MissingStashArtifact(String),

    /// Shell errors from docker subprocess
    SubprocessFailure(i32),
    /// Docker permission gate
    DockerPermissionSafety(String, u32, u32),
    /// Docker image not found
    DockerImageNotFound(String),

    // fetch/update failures
    /// Unspecified install failure
    InstallFailure,
    /// Fetch failure related to backend
    BackendFailure(String),
    /// No version found at same version across `supportedEnvironments`
    NoIntersectedVersion(String),

    // publish errors
    /// Missing release build
    MissingReleaseBuild,
    /// Config missing backend credentials
    MissingBackendCredentials,
    /// Failed upload request to the backend
    UploadFailure(String),

    // upgrade error
    /// Failing to write to our current install prefix
    MissingPrefixPermissions(String),
    /// Failing to validate latest lal version
    UpgradeValidationFailure(String),
}

// Format implementation used when printing an error
impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::Io(ref err) => {
                let knd = err.kind();
                if knd == io::ErrorKind::PermissionDenied {
                    warn!("If you are on norman - ensure you have access to clean ./OUTPUT and \
                           ./INPUT");
                }
                err.fmt(f)
            }
            CliError::Parse(ref err) => err.fmt(f),
            CliError::Hype(ref err) => err.fmt(f),
            CliError::MissingManifest => {
                write!(f,
                       "No manifest.json found - are you at repository toplevel?")
            }
            CliError::ExecutableMissing(ref s) => {
                write!(f,
                       "Please ensure you have `{}` installed on your system first.",
                       s)
            }
            CliError::OutdatedLal(ref o, ref n) => {
                write!(f,
                       "Your version of lal `{}` is too old (<{}). Please `lal upgrade`.",
                       o,
                       n)
            }
            CliError::MissingSslCerts => write!(f, "Missing SSL certificates"),
            CliError::UnmappableRootUser => write!(f, "Root user is not supported for lal builds"),
            CliError::MissingMount(ref s) => write!(f, "Missing mount {}", s),
            CliError::MissingConfig => write!(f, "No ~/.lal/config found"),
            CliError::MissingComponent(ref s) => {
                write!(f, "Component '{}' not found in manifest", s)
            }
            CliError::InvalidComponentName(ref s) => {
                write!(f, "Invalid component name {} - not lowercase", s)
            }
            CliError::ManifestExists => write!(f, "Manifest already exists (use -f to force)"),
            CliError::MissingDependencies => {
                write!(f,
                       "Core dependencies missing in INPUT - try `lal fetch` first")
            }
            CliError::DependencyCycle(ref s) => {
                write!(f, "Cyclical dependencies found for {} in INPUT", s)
            }
            CliError::InvalidVersion(ref s) => {
                write!(f, "Dependency {} using incorrect version", s)
            }
            CliError::ExtraneousDependencies(ref s) => {
                write!(f, "Extraneous dependencies in INPUT ({})", s)
            }
            CliError::MissingLockfile(ref s) => write!(f, "No lockfile found for {}", s),
            CliError::MultipleVersions(ref s) => {
                write!(f, "Depending on multiple versions of {}", s)
            }
            CliError::MultipleEnvironments(ref s) => {
                write!(f, "Depending on multiple environments to build {}", s)
            }
            CliError::EnvironmentMismatch(ref dep, ref env) => {
                write!(f, "Environment mismatch for {} - built in {}", dep, env)
            }
            CliError::NonGlobalDependencies(ref s) => {
                write!(f,
                       "Depending on a custom version of {} (use -s to allow stashed versions)",
                       s)
            }
            CliError::NoSupportedEnvironments => {
                write!(f, "Need to specify supported environments in the manifest")
            }
            CliError::UnsupportedEnvironment => {
                write!(f, "manifest.environment must exist in manifest.supportedEnvironments")
            }
            CliError::MissingEnvironment(ref s) => {
                write!(f, "Environment '{}' not found in ~/.lal/config", s)
            }
            CliError::EnvironmentUnspecified => {
                write!(f, "Environment must be specified for this operation")
            }
            CliError::InvalidBuildConfiguration(ref s) => {
                write!(f, "Invalid build configuration - {}", s)
            }
            CliError::BuildScriptNotExecutable(ref s) => {
                write!(f, "BUILD script at {} is not executable", s)
            }
            CliError::MissingBuildScript => write!(f, "No `BUILD` script found"),
            CliError::MissingScript(ref s) => {
                write!(f, "Missing script '{}' in local folder .lal/scripts/", s)
            }
            CliError::MissingTarball => write!(f, "Tarball missing in PWD"),
            CliError::MissingBuild => write!(f, "No build found in OUTPUT"),
            CliError::InvalidStashName(n) => {
                write!(f,
                       "Invalid name '{}' to stash under - must not be an integer",
                       n)
            }
            CliError::MissingStashArtifact(ref s) => {
                write!(f, "No stashed artifact '{}' found in ~/.lal/cache/stash", s)
            }
            CliError::SubprocessFailure(n) => write!(f, "Process exited with {}", n),
            CliError::DockerPermissionSafety(ref s, u, g) => {
                write!(f,
                       "ID mismatch inside and outside docker - {}; UID and GID are {}:{}",
                       s,
                       u,
                       g)
            }
            CliError::DockerImageNotFound(ref s) => write!(f, "Could not find docker image {}", s),
            CliError::InstallFailure => write!(f, "Install failed"),
            CliError::BackendFailure(ref s) => write!(f, "Backend - {}", s),
            CliError::NoIntersectedVersion(ref s) => {
                write!(f, "No version of {} found across all environments", s)
            }
            CliError::MissingReleaseBuild => write!(f, "Missing release build"),
            CliError::MissingBackendCredentials => {
                write!(f, "Missing backend credentials in ~/.lal/config")
            }
            CliError::MissingPrefixPermissions(ref s) => {
                write!(f,
                       "No write access in {} - consider chowning: `sudo chown -R $USER {}`",
                       s,
                       s)
            }
            CliError::UpgradeValidationFailure(ref s) => {
                write!(f,
                       "Failed to validate new lal version - rolling back ({})",
                       s)
            }
            CliError::UploadFailure(ref up) => write!(f, "Upload failure: {}", up),
        }
    }
}

// Allow io and json errors to be converted to `CliError` in a try! without map_err
impl From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError { CliError::Io(err) }
}

impl From<hyper::Error> for CliError {
    fn from(err: hyper::Error) -> CliError { CliError::Hype(err) }
}

impl From<serde_json::error::Error> for CliError {
    fn from(err: serde_json::error::Error) -> CliError { CliError::Parse(err) }
}

/// Type alias to stop having to type out `CliError` everywhere.
///
/// Most functions can simply add the return type `LalResult<T>` for some `T`,
/// and enjoy the benefit of using `try!` or `?` without having to worry about
/// the many different error types that can arise from using curl, json serializers,
/// file IO, user errors, and potential logic bugs.
pub type LalResult<T> = Result<T, CliError>;
