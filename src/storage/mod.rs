pub use self::traits::{BackendConfiguration, Backend, CachedBackend, Component};

pub use self::artifactory::{ArtifactoryConfig, Credentials, ArtifactoryBackend};

mod traits;
mod artifactory;
mod download;
