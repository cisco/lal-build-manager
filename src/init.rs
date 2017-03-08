use std::env;

use super::{Config, CliError, LalResult};
use core::manifest::*;


/// Generates a blank manifest in the current directory
///
/// This will use the directory name as the assumed default component name
/// Then fill in the blanks as best as possible.
///
/// The function will not overwrite an existing `manifest.json`,
/// unless the `force` bool is set.
pub fn init(cfg: &Config, force: bool, env: &str) -> LalResult<()> {
    cfg.get_container(env.into())?;

    let pwd = env::current_dir()?;
    let last_comp = pwd.components().last().unwrap(); // std::path::Component
    let dirname = last_comp.as_os_str().to_str().unwrap();

    let mpath = ManifestLocation::identify(&pwd);
    if !force && mpath.is_ok() {
        return Err(CliError::ManifestExists);
    }

    // we are allowed to overwrite or write a new manifest if we are here
    // always create new manifests in new default location
    create_lal_subdir(&pwd)?; // create the `.lal` subdir if it's not there already
    Manifest::new(dirname, env, ManifestLocation::default().as_path(&pwd)).write()?;

    // if the manifest already existed, warn about this now being placed elsewhere
    if let Ok(ManifestLocation::RepoRoot) = mpath {
        warn!("Created manifest in new location under .lal");
        warn!("Please delete the old manifest - it will not be read anymore");
    }

    Ok(())
}
