pub use self::traits::{BackendConfiguration, Backend, CachedBackend, Component};

pub use self::artifactory::{ArtifactoryConfig, Credentials, ArtifactoryBackend};
pub use self::github::{GithubConfig, GithubCredentials, GithubBackend};

// Some special exports for lal upgrade - canonical releases are on artifactory atm
#[cfg(feature = "upgrade")]
pub use self::artifactory::{LatestLal, get_latest_lal_version, http_download_to_path};

mod traits;
mod download;
mod artifactory;
mod github;

#[cfg(feature = "progress")]
mod progress;
