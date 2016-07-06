#[macro_use]
extern crate clap;
use clap::Shell;
use std::path::Path;
//use std::fs; TODO: move output file bash.sh -> lal.complete.sh when ready

include!("src/cli.rs");

fn main() {
     let mut app = application();
     app.gen_completions("lal", Shell::Bash, Path::new("."));
 }
