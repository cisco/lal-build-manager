use std::fs;
use std::path::Path;
use std::env;

use init;
use errors::CliError;

pub fn verify() -> Result<(), CliError> {
    // 1. `manifest.json` exists in `$PWD` and is valid JSON
    let m = try!(init::read_manifest());

    // 2. dependencies in `INPUT` match `manifest.json`.
    let input = Path::new(&env::current_dir().unwrap()).join("INPUT");
    let mut deps = vec![];
    for entry in try!(fs::read_dir(&input)) {
        let pth = try!(entry).path();
        if pth.is_dir() {
            let component = pth.to_str().unwrap().split("/").last().unwrap();
            deps.push(component.to_string());
        }
    }
    debug!("Found the following deps in INPUT: {:?}", deps);
    for (d, v) in m.dependencies {
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

#[cfg(test)]
mod tests {
    use verify;
    use install;
    use init;
    use configure;

    #[test]
    #[ignore]
    fn fails_on_missing_dir() {
        // Can't really run this consistenly unless create an order of tests
        // if they're all in separate files all messing with INPUT it's silly
        let manifest = init::read_manifest();
        assert_eq!(manifest.is_ok(), true);
        let mf = manifest.unwrap();
        let config = configure::current_config();
        assert_eq!(config.is_ok(), true);
        let cfg = config.unwrap();

        let r = verify::verify();
        assert_eq!(r.is_err(), true);
        install::install_all(mf, cfg, false);
        let r = verify::verify();
        assert_eq!(r.is_ok(), true);
    }
}
