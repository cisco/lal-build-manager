use std::process::Command;
use std::vec::Vec;

use super::{StickyOptions, LalResult, CliError, Container, Config};

/// Pull the current environment from docker
pub fn update(container: &Container, env: &str) -> LalResult<()> {
    info!("Updating {} container", env);
    let args: Vec<String> = vec!["pull".into(), format!("{}", container)];
    trace!("Docker pull {}", container);
    let s = Command::new("docker").args(&args).status()?;
    trace!("Exited docker");
    if !s.success() {
        return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
    }
    Ok(())
}

/// Creates and sets the environment in the local .lal/opts file
pub fn set(opts_: &StickyOptions, cfg: &Config, env: &str) -> LalResult<()> {
    if !cfg.environments.contains_key(env) {
        return Err(CliError::MissingEnvironment(env.into()));
    }
    // mutate a temporary copy - lal binary is done after this function anyway
    let mut opts = opts_.clone();
    opts.env = Some(env.into());
    opts.write()?;
    Ok(())
}

/// Clears the local .lal/opts file
pub fn clear() -> LalResult<()> {
    let _ = StickyOptions::delete_local();
    Ok(())
}
