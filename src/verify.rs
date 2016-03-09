use std::fs;
use std::path::Path;
use std::env;

use init;
use errors::{CliError, LalResult};

pub fn verify() -> LalResult<()> {
    // 1. `manifest.json` exists in `$PWD` and is valid JSON
    let m = try!(init::read_manifest());

    // 2. dependencies in `INPUT` match `manifest.json`.
    let input = Path::new(&env::current_dir().unwrap()).join("INPUT");
    if !input.is_dir() && m.dependencies.len() == 0 {
        return Ok(()); // nothing to verify - so accept a missing directory
    }
    let mut deps = vec![];
    for entry in try!(fs::read_dir(&input)) {
        let pth = try!(entry).path();
        if pth.is_dir() {
            let component = pth.to_str().unwrap().split("/").last().unwrap();
            deps.push(component.to_string());
        }
    }
    debug!("Found the following deps in INPUT: {:?}", deps);
    for (d, _) in m.dependencies {
        debug!("Verifying dependency from manifest: {}", d);
        if !deps.contains(&d) {
            warn!("Dependency {} not found in INPUT", d);
            return Err(CliError::MissingDependencies);
        }
    }

    // 3. the dependency tree is flat.
    // TODO:

    // 4. `INPUT` contains only global dependencies.
    // TODO:
    Ok(())
}
