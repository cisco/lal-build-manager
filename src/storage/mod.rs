pub use self::traits::{Backend, BackendConfiguration, CachedBackend, Component};

pub use self::artifactory::{ArtifactoryBackend, ArtifactoryConfig, Credentials};
pub use self::local::{LocalBackend, LocalConfig};

// Some special exports for lal upgrade - canonical releases are on artifactory atm
#[cfg(feature = "upgrade")]
pub use self::artifactory::{get_latest_lal_version, http_download_to_path, LatestLal};

mod artifactory;
mod download;
mod local;
mod traits;

#[cfg(feature = "progress")]
mod progress;
