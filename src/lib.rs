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

extern crate curl;
extern crate rustc_serialize;
extern crate regex;
extern crate tar;
extern crate flate2;
extern crate ansi_term;
#[macro_use]
extern crate log;
extern crate walkdir;

// re-exports
pub use init::Manifest;
pub use configure::Config;
pub use errors::{LalResult, CliError};
pub use util::lockfile::Lockfile;

mod util;
pub mod errors;
pub mod configure;
pub mod init;
pub mod shell;
pub mod build;
pub mod install;
pub mod verify;
pub mod cache;
pub mod status;
