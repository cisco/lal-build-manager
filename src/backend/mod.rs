pub use self::backend::{Backend, Component};

pub use self::artifactory::{ArtifactoryConfig, Credentials, Artifactory};

// TODO: cleanup
pub use self::download::*;

mod backend;
mod download;

/// Backend implementation
pub mod artifactory;
