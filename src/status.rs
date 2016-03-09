use std::fs;
use std::path::Path;
use std::env;
use ansi_term::Colour;

use init::{self, Manifest};
use errors::{CliError, LalResult};

fn get_installed() -> LalResult<Vec<String>> {
    let input = Path::new(&env::current_dir().unwrap()).join("INPUT");
    let mut deps = vec![];
    for entry in try!(fs::read_dir(&input)) {
        let pth = try!(entry).path();
        if pth.is_dir() {
            let component = pth.to_str().unwrap().split("/").last().unwrap();
            deps.push(component.to_string());
        }
    }
    Ok(deps)
}

// dumb helper to print a one-level tree
fn print_as_tree(root: &str, xs: Vec<String>) {
    let len = xs.len();
    println!("{}", root);
    let mut i = 0;
    for name in xs {
        i += 1;
        let branch_str = format!("{}", if i == len { "└" } else { "├" });
        println!("{}── {}", branch_str, name);
    }
}

pub fn status(manifest: Manifest) -> LalResult<()> {
    let deps = try!(get_installed());
    let saved_deps = init::merge_dependencies(&manifest);

    let mut res = vec![];
    let mut error = None;

    // figure out status of saved deps
    for (d, v) in saved_deps.clone() {
        let missing = !deps.contains(&d);
        let is_dev = manifest.devDependencies.contains_key(&d);

        let mut extra_str = "".to_string();
        if missing && !is_dev {
            error = Some(CliError::MissingDependencies);
            extra_str = Colour::Red.paint("(missing)").to_string();
        } else if missing {
            extra_str = Colour::Yellow.paint("(missing)").to_string();
        } else if is_dev {
            extra_str = "(dev)".to_string();
        }
        // TODO: cross reference version with the installed ones!

        let format_str = format!("{}@{} {}", d, v, extra_str);
        res.push(format_str);
    }
    // figure out status of installed deps (may find extraneous ones)
    for name in deps {
        if !saved_deps.contains_key(&name) {
            let extra_str = Colour::Green.paint("(extraneous)").to_string();
            let version = "?"; // TODO: get version from INPUT as well..
            res.push(format!("{}@{} {}", name, version, extra_str));
            // this dependency is neither a dependency nor a devDependency!
            error = Some(CliError::ExtraneousDependencies);
        }
    }

    print_as_tree(&manifest.name, res);

    // Return one of the errors as the main one (no need to vectorize these..)
    if error.is_some() {
        return Err(error.unwrap());
    }
    Ok(())
}
