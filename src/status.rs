use ansi_term::Colour;
use Manifest;
use errors::{CliError, LalResult};
use util::input;

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

/// Prints a fancy dependency tree of `./INPUT` to stdout.
///
/// This is the quick version information of what you currently have in `./INPUT`.
/// It prints the tree and highlights versions, as well as both missing and extraneous
/// dependencies in `./INPUT`.
///
/// TODO: This function should be extended to also print the FULL tree by analysing
/// lockfiles.
///
/// It is not intended as a verifier, but will nevertheless produce a summary at the end.
pub fn status(manifest: Manifest) -> LalResult<()> {
    let mut res = vec![];
    let mut error = None;

    for (d, dep) in try!(input::analyze_full(&manifest)) {
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

        res.push(format!("{}@{} {}", d, dep.version, notes));
    }
    print_as_tree(&manifest.name, res);

    // Return one of the errors as the main one (no need to vectorize these..)
    if error.is_some() {
        return Err(error.unwrap());
    }
    Ok(())
}
