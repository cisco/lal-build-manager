use std::process::Command;
use std::fs;
use std::io::prelude::{Read, Write};
use std::path::Path;
use std::vec::Vec;
use rustc_serialize::json;

use {Container, Config, CliError, LalResult};

/// Representation of .lalopts
///
/// This contains the currently supported, directory-wide, sticky options.
#[derive(RustcDecodable, RustcEncodable, Clone, Default)]
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
        try!(try!(fs::File::open(&opts_path)).read_to_string(&mut opts_data));
        let res = try!(json::decode(&opts_data));
        Ok(res)
    }

    /// Overwrite `.lalopts` with current settings
    pub fn write(&self, silent: bool) -> LalResult<()> {
        let opts_path = Path::new(".lalopts");
        let encoded = json::as_pretty_json(self);

        let mut f = try!(fs::File::create(&opts_path));
        try!(write!(f, "{}\n", encoded));
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
        Ok(try!(fs::remove_file(&opts_path)))
    }
}


pub fn update(container: &Container, env: &str) -> LalResult<()> {
    info!("Updating {} container", env);
    let args: Vec<String> = vec!["pull".into(), format!("{}", container)];
    trace!("Docker pull {}", container);
    let s = try!(Command::new("docker").args(&args).status());
    trace!("Exited docker");
    if !s.success() {
        return Err(CliError::SubprocessFailure(s.code().unwrap_or(1001)));
    }
    Ok(())
}

pub fn set(opts_: &StickyOptions, cfg: &Config, env: &str) -> LalResult<()> {
    if !cfg.environments.contains_key(env) {
        return Err(CliError::MissingEnvironment(env.into()));
    }
    // mutate a temporary copy - lal binary is done after this function anyway
    let mut opts = opts_.clone();
    opts.env = Some(env.into());
    try!(opts.write(false));
    Ok(())
}

pub fn clear() -> LalResult<()> {
    try!(StickyOptions::delete_local());
    Ok(())
}
