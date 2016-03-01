use std::collections::HashMap;
use std::io;
use std::process;
use rustc_serialize::json;

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Manifest {
    name: String,
    version: String,
    dependencies: HashMap<String, String>,
    devDependencies: HashMap<String, String>,
}

pub fn init(force: bool) -> io::Result<Manifest> {
    use std::env;
    use std::path::Path;
    use std::fs::File;
    use std::io::prelude::*;

    let pwd = env::current_dir().unwrap();
    let last_comp = pwd.components().last().unwrap(); // std::path::Component
    let dirname = last_comp.as_os_str().to_str().unwrap();

    let manifest = Manifest {
        name: dirname.to_string(),
        version: "0".to_string(),
        dependencies: HashMap::new(),
        devDependencies: HashMap::new(),
    };

    let encoded = json::as_pretty_json(&manifest);

    let manifest_path = Path::new(&pwd).join("manifest.json");
    if !force && manifest_path.exists() {
        println!("manifest.json already exists, stopping.");
        println!("Use -f to overwrite");
        process::exit(1);
    }
    let mut f = try!(File::create(&manifest_path));
    try!(write!(f, "{}\n", encoded));

    println!("Wrote manifest {}: \n{}", manifest_path.display(), encoded);
    return Ok(manifest.clone());
}
