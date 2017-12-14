/// This file contains all the hidden `lal list-*` subcommands
/// If you are looking for `lal ls` go to status.rs

use super::{Manifest, Config, LalResult};

/// Print the buildable components from the `Manifest`
pub fn buildables(manifest: &Manifest) -> LalResult<()> {
    for k in manifest.components.keys() {
        println!("{}", k);
    }
    Ok(())
}

/// Print the supported environments from the `Manifest`
pub fn supported_environments(manifest: &Manifest) -> LalResult<()> {
    for env in &manifest.supportedEnvironments {
        println!("{}", env);
    }
    Ok(())
}

/// Print the available configurations for a buildable Component
pub fn configurations(component: &str, manifest: &Manifest) -> LalResult<()> {
    let component_settings = match manifest.components.get(component) {
        Some(c) => c,
        None => return Ok(()), // invalid component - but this is for completion
    };
    for c in &component_settings.configurations {
        println!("{}", c);
    }
    Ok(())
}

/// Print the configured environments from the config
pub fn environments(cfg: &Config) -> LalResult<()> {
    for k in cfg.environments.keys() {
        println!("{}", k);
    }
    Ok(())
}

/// Print the dependencies from the manifest
pub fn dependencies(mf: &Manifest, core: bool) -> LalResult<()> {
    let deps = if core { mf.dependencies.clone() } else { mf.all_dependencies() };
    for k in deps.keys() {
        println!("{}", k);
    }
    Ok(())
}
