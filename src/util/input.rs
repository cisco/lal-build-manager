use std::fs;
use std::path::Path;
use std::env;
use std::collections::HashMap;

use errors::LalResult;

// TODO: this is not all the information we need everywhere..
// This is just all the deps, no distiction between dev and core
pub fn analyze() -> LalResult<HashMap<String, String>> {
    let cwd = try!(env::current_dir());
    let input = Path::new(&cwd).join("INPUT");

    let mut deps = HashMap::new();
    if !input.is_dir() {
        return Ok(deps);
    }

    for entry in try!(fs::read_dir(&input)) {
        let pth = try!(entry).path();
        if pth.is_dir() {
            let component = pth.to_str().unwrap().split("/").last().unwrap();
            deps.insert(component.to_string(), "experimental".to_string());
        }
    }
    Ok(deps)
}
