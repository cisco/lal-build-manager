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

extern crate curl;
extern crate rustc_serialize;
extern crate regex;
extern crate tar;
extern crate flate2;
extern crate ansi_term;
#[macro_use]
extern crate log;
extern crate walkdir;
extern crate semver;
extern crate chrono;
extern crate filetime;

// re-exports
pub use util::lockfile::Lockfile;
pub use errors::{LalResult, CliError};
pub use build::{build, build_list};
// pub use multibuild::multibuild;
pub use configure::{configure, Config};
pub use init::{init, Manifest};
pub use shell::{shell, docker_run};
pub use install::{fetch, update, remove, export};
pub use status::status;
pub use verify::verify;
pub use cache::{stash, clean};
pub use upgrade::upgrade_check;

mod util;
mod errors;
mod configure;
mod init;
mod shell;
mod build;
// mod multibuild;
mod install;
mod verify;
mod cache;
mod status;
mod upgrade;
