use std::fs;
use std::path::Path;

use super::{CliError, LalResult, Manifest};

/// Remove specific components from `./INPUT` and the manifest.
///
/// This takes multiple components strings (without versions), and if the component
/// is found in `./INPUT` it is deleted.
///
/// If one of `save` or `savedev` was set, `manifest.json` is also updated to remove
/// the specified components from the corresponding dictionary.
pub fn remove(manifest: &Manifest, xs: Vec<String>, save: bool, savedev: bool) -> LalResult<()> {
    debug!("Removing dependencies {:?}", xs);

    // remove entries in xs from manifest.
    if save || savedev {
        let mut mf = manifest.clone();
        let mut hmap = if save { mf.dependencies.clone() } else { mf.devDependencies.clone() };
        for component in xs.clone() {
            // We could perhaps allow people to just specify ANY dependency
            // and have a generic save flag, which we could infer from
            // thus we could modify both maps if listing many components

            // This could work, but it's not currently what install does, so not doing it.
            // => all components uninstalled from either dependencies, or all from devDependencies
            // if doing multiple components from different maps, do multiple calls
            if !hmap.contains_key(&component) {
                return Err(CliError::MissingComponent(component.to_string()));
            }
            debug!("Removing {} from manifest", component);
            hmap.remove(&component);
        }
        if save {
            mf.dependencies = hmap;
        } else {
            mf.devDependencies = hmap;
        }
        info!("Updating manifest with removed dependencies");
        mf.write()?;
    }

    // delete the folder (ignore if the folder does not exist)
    let input = Path::new("./INPUT");
    if !input.is_dir() {
        return Ok(());
    }
    for component in xs {
        let pth = Path::new(&input).join(&component);
        if pth.is_dir() {
            debug!("Deleting INPUT/{}", component);
            fs::remove_dir_all(&pth)?;
        }
    }
    Ok(())
}
