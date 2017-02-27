use std::process::Command;
use std::fs;
use std::io::prelude::{Read, Write};
use std::path::Path;
use std::vec::Vec;
use serde_json;

use super::{Container, Config, CliError, LalResult};

/// Representation of .lalopts
///
/// This contains the currently supported, directory-wide, sticky options.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct StickyOptions {
    /// Environment to be used implicitally instead of the default
    pub env: Option<String>,
}

impl StickyOptions {
    /// Initialize a StickyOptions with defaults
    pub fn new() -> StickyOptions {
        Default::default()
    }
    /// Read and deserialize a StickyOptions from `.lalopts`
    pub fn read() -> LalResult<StickyOptions> {
        let opts_path = Path::new(".lalopts");
        if !opts_path.exists() {
            return Ok(StickyOptions::default()); // everything off
        }
        let mut opts_data = String::new();
        fs::File::open(&opts_path)?.read_to_string(&mut opts_data)?;
        let res = serde_json::from_str(&opts_data)?;
        Ok(res)
    }

    /// Overwrite `.lalopts` with current settings
    pub fn write(&self, silent: bool) -> LalResult<()> {
        let opts_path = Path::new(".lalopts");
        let encoded = serde_json::to_string_pretty(self)?;

        let mut f = fs::File::create(&opts_path)?;
        write!(f, "{}\n", encoded)?;
        if silent {
            debug!("Wrote {}: \n{}", opts_path.display(), encoded);
        } else {
            info!("Wrote {}: \n{}", opts_path.display(), encoded);
        }
        Ok(())
    }
    /// Delete local `.lalopts`
    pub fn delete_local() -> LalResult<()> {
        let opts_path = Path::new(".lalopts");
        Ok(fs::remove_file(&opts_path)?)
    }
}

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

/// Creates and sets the environment in the local .lalopts file
pub fn set(opts_: &StickyOptions, cfg: &Config, env: &str) -> LalResult<()> {
    if !cfg.environments.contains_key(env) {
        return Err(CliError::MissingEnvironment(env.into()));
    }
    // mutate a temporary copy - lal binary is done after this function anyway
    let mut opts = opts_.clone();
    opts.env = Some(env.into());
    opts.write(false)?;
    Ok(())
}

/// Clears the local .lalopts file
pub fn clear() -> LalResult<()> {
    let _ = StickyOptions::delete_local();
    Ok(())
}
