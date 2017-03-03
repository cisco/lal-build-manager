#![warn(missing_docs)]

//! This is the rust doc for the `lal` *library* - what the `lal` *binary*
//! depends on to do all the work. This documentation is likely only of use to you
//! if you need to know the internals of `lal` for figuring out how to modify it.
//!
//! For documentation on using the lal binary, see
//! [the main readme](https://sqbu-github.cisco.com/Edonus/lal/blob/master/README.md)
//!
//! ## Testing
//! The library contains all the logic because the binary is only an argument parser,
//! and elaborate decision making engine to log, call one of the libraries functions,
//! then simply `process::exit`.
//! Tests do not cover the binary part, because these failures would be trivially
//! detectable, and also require a subprocess type of testing. Tests instead
//! cover a couple of common use flows through the library.
//!
//! ## Spec
//! This library performs the basic actions needed to adhere to the
//! [SPEC.md](https://sqbu-github.cisco.com/Edonus/lal/blob/master/SPEC.md).
//!
//! ## Dependencies
//! This tool depends on the rust ecosystem and their crates. Dependencies referenced
//! explicitly or implicitly is listed on the left of this page.

#[macro_use]
extern crate hyper;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate regex;
extern crate tar;
extern crate flate2;
extern crate ansi_term;
extern crate sha1;
#[macro_use]
extern crate log;
extern crate walkdir;
extern crate semver;
extern crate chrono;
extern crate filetime;
extern crate rand;

// re-exports
mod core;
pub use core::*;

mod backend;
pub use backend::*;

pub use build::{build, build_list, configuration_list, BuildOptions};
pub use configure::{configure, env_list};
pub use init::{init, dep_list};
pub use shell::{shell, docker_run, script, DockerRunFlags};
pub use install::{fetch, update, update_all, remove, export};
pub use status::status;
pub use verify::verify;
pub use stash::{stash, clean};
pub use upgrade::upgrade_check;
pub use query::query;
pub use publish::publish;

/// Env module for env subcommand
pub mod env;

mod configure;
mod init;
mod shell;
mod build;
mod query;
mod install;
mod verify;
mod stash;
mod status;
mod upgrade;
mod publish;
