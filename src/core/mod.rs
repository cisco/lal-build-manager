#![allow(missing_docs)]

// TODO: document better this stuff, most of this stuff is actually mostly documented

pub use self::errors::{CliError, LalResult};
pub use self::manifest::{Manifest, ComponentConfiguration, ManifestLocation};
pub use self::lockfile::{Lockfile, Container};
pub use self::config::{Config, ConfigDefaults, Artifactory, Credentials, Mount};

mod config;
mod errors;
mod lockfile;
pub mod manifest;

pub mod input;
pub mod artifactory;
