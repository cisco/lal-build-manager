use std::fmt;
use std::io;
use rustc_serialize::json;

#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    Parse(json::DecoderError),
    // main errors (via init and configure)
    MissingManifest,
    MissingConfig,
    MissingComponent(String), // in manifest component list
    ManifestExists, // when trying to init over existing one

    // status/verify errors
    MissingDependencies,
    ExtraneousDependencies,

    // build errors
    InvalidBuildConfiguration(String),

    // cache errors
    MissingTarball,
    MissingBuild,

    // shell errors
    SubprocessFailure(i32),

    // Install failures
    InstallFailure, // generic catch all
    GlobalRootFailure(&'static str), // fetch failure related to globalroot
    ArtifactoryFailure(&'static str), // fetch failure related to artifactory
}
pub type LalResult<T> = Result<T, CliError>;

// Format implementation used when printing an error
impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::Io(ref err) => err.fmt(f),
            CliError::Parse(ref err) => err.fmt(f),
            CliError::MissingManifest => write!(f, "No manifest.json found"),
            CliError::MissingConfig => write!(f, "No ~/.lal/lalrc found"),
            CliError::MissingComponent(ref s) => write!(f, "Component '{}' not found in manifest", s),
            CliError::ManifestExists => write!(f, "Manifest already exists (use -f to force)"),
            CliError::MissingDependencies => write!(f, "Core dependencies missing in INPUT"),
            CliError::ExtraneousDependencies => write!(f, "Extraneous dependencies in INPUT"),
            CliError::InvalidBuildConfiguration(ref s) => {
                write!(f, "Invalid build configuration - {}", s)
            }
            CliError::MissingTarball => write!(f, "Tarball missing in PWD"),
            CliError::MissingBuild => write!(f, "No build found in OUTPUT"),
            CliError::SubprocessFailure(n) => write!(f, "Process exited with {}", n),
            CliError::InstallFailure => write!(f, "Install failed"),
            CliError::GlobalRootFailure(ref s) => write!(f, "Globalroot - {}", s),
            CliError::ArtifactoryFailure(ref s) => write!(f, "Artifactory - {}", s),
        }
    }
}

// Allow io and json errors to be converted to CliError in a try! without map_err
impl From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError {
        CliError::Io(err)
    }
}

impl From<json::DecoderError> for CliError {
    fn from(err: json::DecoderError) -> CliError {
        CliError::Parse(err)
    }
}
