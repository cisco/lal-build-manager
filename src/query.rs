use std::io::{self, Write};

use backend::Backend;
use super::LalResult;

/// Prints a list of versions associated with a component
pub fn query(backend: &Backend, env: Option<&str>, component: &str) -> LalResult<()> {
    let vers = backend.get_versions(component, env)?;
    for v in vers {
        println!("{}", v);
        // needed because sigpipe handling is broken for stdout atm
        // see #36 - can probably be taken out in rust 1.16 or 1.17
        // if `lal query media-engine | head` does not crash
        if io::stdout().flush().is_err() {
            return Ok(());
        }
    }
    Ok(())
}
