#![warn(missing_docs)]

//! This is the rust doc for the `lal` *library* - what the `lal` *binary*
//! depends on to do all the work. This documentation is likely only of use to you
//! if you need to know the internals of `lal` for figuring out how to modify it.
//!
//! ## Testing
//! The library contains all the logic because the binary is only an argument parser,
//! and elaborate decision making engine to log, call one of the libraries functions,
//! then simply `process::exit`.
//! Tests do not cover the binary part, because these failures would be trivially
//! detectable, and also require a subprocess type of testing. Tests instead
//! cover a couple of common use flows through the library.
//!
//!
//! ## Dependencies
//! This tool depends on the rust ecosystem and their crates. Dependencies referenced
//! explicitly or implicitly is listed on the left of this page.

#[macro_use]
extern crate hyper;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;

// re-exports
mod core;
pub use crate::core::*;

mod storage;
pub use crate::storage::*;

/// Channel module for channel subcommand
pub mod channel;
/// Env module for env subcommand (which has further subcommands)
pub mod env;
/// List module for all the list-* subcommands
pub mod list;
/// Propagation module with all structs describing the steps
pub mod propagate;
/// Verification of state
pub mod verify;

// lift most other pub functions into our libraries main scope
// this avoids having to type lal::build::build in tests and main.rs
pub use crate::build::{build, BuildOptions};
pub use crate::clean::clean;
pub use crate::configure::configure;
pub use crate::export::export;
pub use crate::fetch::fetch;
pub use crate::init::init;
pub use crate::publish::publish;
pub use crate::query::query;
pub use crate::remove::remove;
pub use crate::shell::{docker_run, script, shell, DockerRunFlags, ShellModes};
pub use crate::stash::stash;
pub use crate::status::status;
pub use crate::update::{update, update_all};

mod build;
mod clean;
mod configure;
mod export;
mod fetch;
mod init;
mod publish;
mod query;
mod remove;
mod shell;
mod stash;
mod status;
mod update;

#[cfg(feature = "upgrade")]
pub use crate::upgrade::upgrade;
#[cfg(feature = "upgrade")]
mod upgrade;
