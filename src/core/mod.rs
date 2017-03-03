pub use self::errors::{CliError, LalResult};
pub use self::manifest::{Manifest, ComponentConfiguration, ManifestLocation};
pub use self::lockfile::{Lockfile, Container};
pub use self::config::{Config, ConfigDefaults, Mount};
pub use self::sticky::StickyOptions;

mod config;
mod errors;
mod lockfile;
mod sticky;

/// Manifest module can be used directly
pub mod manifest;

/// Simple INPUT folder analyzer module can be used directly
pub mod input;

/// Simple OUTPUT folder helper module
pub mod output;
