use std::fmt;
use std::io;
use rustc_serialize::json;

#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    Parse(json::DecoderError),
    MissingManifest,
    MissingConfig,
    MissingDependencies,
    ExtraneousDependencies,
    MissingTarball,
    SubprocessFailure(i32),
    InstallFailure,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::Io(ref err) => err.fmt(f),
            CliError::Parse(ref err) => err.fmt(f),
            CliError::MissingManifest => write!(f, "No manifest.json found"),
            CliError::MissingConfig => write!(f, "No ~/.lal/lalrc found"),
            CliError::MissingDependencies => write!(f, "Dependencies missing in INPUT"),
            CliError::ExtraneousDependencies => write!(f, "Extraneous dependencies in INPUT"),
            CliError::MissingTarball => write!(f, "Tarball missing in PWD"),
            CliError::SubprocessFailure(n) => write!(f, "Process exited with {}", n),
            CliError::InstallFailure => write!(f, "Install failed"),
        }
    }
}

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
