use std::fs;
use std::env;
use std::io::prelude::{Read, Write};
use std::path::Path;
use serde_json;

use super::LalResult;
use manifest::create_lal_subdir;

/// Representation of .lal/opts
///
/// This contains the currently supported, directory-wide, sticky options.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct StickyOptions {
    /// Environment to be used implicitally instead of the default
    pub env: Option<String>,
}

impl StickyOptions {
    /// Initialize a StickyOptions with defaults
    pub fn new() -> StickyOptions { Default::default() }
    /// Read and deserialize a StickyOptions from `.lal/opts`
    pub fn read() -> LalResult<StickyOptions> {
        let opts_path = Path::new(".lal/opts");
        if !opts_path.exists() {
            return Ok(StickyOptions::default()); // everything off
        }
        let mut opts_data = String::new();
        fs::File::open(&opts_path)?.read_to_string(&mut opts_data)?;
        let res = serde_json::from_str(&opts_data)?;
        Ok(res)
    }

    /// Overwrite `.lal/opts` with current settings
    pub fn write(&self) -> LalResult<()> {
        let pwd = env::current_dir()?;
        create_lal_subdir(&pwd)?; // create the `.lal` subdir if it's not there already
        let opts_path = Path::new(".lal/opts");
        let encoded = serde_json::to_string_pretty(self)?;

        let mut f = fs::File::create(&opts_path)?;
        write!(f, "{}\n", encoded)?;
        debug!("Wrote {}: \n{}", opts_path.display(), encoded);
        Ok(())
    }
    /// Delete local `.lal/opts`
    pub fn delete_local() -> LalResult<()> {
        let opts_path = Path::new(".lal/opts");
        Ok(fs::remove_file(&opts_path)?)
    }
}
