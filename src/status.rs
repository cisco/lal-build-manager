use ansi_term::Colour;
use Manifest;
use errors::{CliError, LalResult};
use util::input;
use super::Lockfile;

fn status_recurse(dep: &String, lf: &Lockfile, n: usize, parent_indent: Vec<bool>) {
    assert_eq!(dep, &lf.name);
    let len = lf.dependencies.len();
    let mut i = 0;
    for (k, sublock) in &lf.dependencies {
        let has_children = sublock.dependencies.len() > 0;
        let fork_char = if has_children { "┬" } else { "─" };
        let is_last = i == len - 1;
        let turn_char = if is_last { "└" } else { "├" };

        let ws: String = parent_indent.iter().fold(String::new(), |res, &ws_only| {
            res + &format!("{}", if ws_only { "  " } else { "│ " })
        });

        println!("│ {}{}─{} {}", ws, turn_char, fork_char, k);

        let mut next_indent = parent_indent.clone();
        next_indent.push(is_last);

        status_recurse(&k, &sublock, n + 1, next_indent);
        i += 1;
    }
}

/// Prints a fancy dependency tree of `./INPUT` to stdout.
///
/// This is the quick version information of what you currently have in `./INPUT`.
/// It prints the tree and highlights versions, as well as both missing and extraneous
/// dependencies in `./INPUT`.
///
/// If the full flag is given, then the full dependency tree is also spliced in
/// from lockfile data.
///
/// It is not intended as a verifier, but will nevertheless produce a summary at the end.
pub fn status(manifest: &Manifest, full: bool) -> LalResult<()> {
    let mut error = None;

    let lf = if full {
        try!(Lockfile::default().populate_from_input())
    } else {
        Lockfile::default()
    };

    println!("{}", manifest.name);
    let mut i = 0;
    let deps = try!(input::analyze_full(&manifest));
    let len = deps.len();
    for (d, dep) in deps {
        let notes = if dep.missing && !dep.development {
            error = Some(CliError::MissingDependencies);
            Colour::Red.paint("(missing)").to_string()
        } else if dep.missing {
            Colour::Yellow.paint("(missing)").to_string()
        } else if dep.development {
            "(dev)".to_string()
        } else if dep.extraneous {
            error = Some(CliError::ExtraneousDependencies);
            Colour::Green.paint("(extraneous)").to_string()
        } else {
            "".to_string()
        };
        // list children in --full mode
        // NB: missing deps will not be populatable
        let has_children = full && !dep.missing &&
                           lf.dependencies.get(&dep.name).unwrap().dependencies.len() > 0;
        let fork_char = if has_children { "┬" } else { "─" };
        let is_last = i == len - 1;
        let turn_char = if is_last { "└" } else { "├" };

        // first level deps are formatted with more metadata
        let level1 = format!("{}@{} {}", d, dep.version, notes);
        println!("{}─{} {}", turn_char, fork_char, level1);

        if has_children {
            trace!("Attempting to get {} out of lockfile deps {:?}",
                   dep.name,
                   lf.dependencies);
            // dep unwrap relies on populate_from_input try! reading all lockfiles earlier
            let sub_lock = lf.dependencies.get(&dep.name).unwrap();
            status_recurse(&dep.name, sub_lock, 1, vec![]);
        }
        i += 1;
    }

    // Return one of the errors as the main one (no need to vectorize these..)
    if error.is_some() {
        return Err(error.unwrap());
    }
    Ok(())
}
