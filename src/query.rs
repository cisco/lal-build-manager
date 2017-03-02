use std::io::{self, Write};

use backend::artifactory::get_latest_versions;
use super::{LalResult, Config};

/// Prints a list of versions associated with a component
pub fn query(cfg: &Config, component: &str) -> LalResult<()> {
    let vers = get_latest_versions(&cfg.artifactory, component)?;
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
