use std::io::{self, Write};

use super::{CliError, LalResult};
use crate::channel::Channel;
use crate::storage::Backend;

/// Prints a list of versions associated with a component
pub fn query(
    backend: &dyn Backend,
    env: Option<&str>,
    channel: Option<&str>,
    component: &str,
    last: bool,
) -> LalResult<()> {
    let channel = Channel::from_option(&channel);
    if component.to_lowercase() != component {
        return Err(CliError::InvalidComponentName(component.into()));
    }
    let env = match env {
        None => {
            error!("query is no longer allowed without an explicit environment. Specify an environment by passing the -e flag to lal");
            return Err(CliError::EnvironmentUnspecified);
        }
        Some(e) => e,
    };

    if last {
        let ver = backend.get_latest_version(component, env, &channel)?;
        println!("{}", ver);
    } else {
        let vers = backend.get_versions(component, env, &channel)?;
        for v in vers {
            println!("{}", v);
            // needed because sigpipe handling is broken for stdout atm
            // see #36 - can probably be taken out in rust 1.16 or 1.17
            // if `lal query media-engine | head` does not crash
            if io::stdout().flush().is_err() {
                return Ok(());
            }
        }
    }
    Ok(())
}
