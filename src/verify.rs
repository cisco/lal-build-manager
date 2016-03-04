use std::io;
use std::fs;
use std::path::Path;
use std::env;
use std::io::{Error, ErrorKind};

use init;

pub fn verify() -> io::Result<()> {
    // 1. `manifest.json` exists in `$PWD` and is valid JSON
    let m = try!(init::read_manifest()); // TODO: better error output

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
            let reason = format!("Dependency {} not found in INPUT", d);
            return Err(Error::new(ErrorKind::Other, reason));
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

    #[test]
    fn fails_on_missing_dir() {
        let r = verify::verify();
        assert_eq!(r.is_err(), true);
    }
}
