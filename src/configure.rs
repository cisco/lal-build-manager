use std::io;
use rustc_serialize::json;

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Config {
    registry: String,
    cache: String,
    target: String,
    container: String,
}

fn prompt(name: &str, default: String) -> String {
    use std::io::{self, Write};
    print!("Default {}: ({}) ", name, &default);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            if n > 1 {
                // more than just a newline character (which we strip)
                return (&input[0..n - 1]).to_string();
            }
        }
        Err(error) => println!("error: {}", error),
    }
    return default;
}

pub fn configure() -> io::Result<Config> {
    use std::path::Path;
    use std::fs::File;
    use std::env;
    use std::io::prelude::*;

    let mut cfg = Config {
        registry: "http://localhost".to_string(),
        cache: "~/.lal/cache".to_string(),
        target: "ncp.amd64".to_string(),
        container: "edonusdevelopers/centos_build".to_string(),
    };

    let home = env::home_dir().unwrap(); // crash if no $HOME
    let cfg_path = Path::new(&home).join(".lal/lalrc");

    // Prompt for values:
    cfg.registry = prompt("registry", cfg.registry);
    cfg.cache = prompt("cache", cfg.cache);
    cfg.target = prompt("target", cfg.target);
    cfg.container = prompt("container", cfg.container);

    // Encode
    let encoded = json::as_pretty_json(&cfg);

    let mut f = try!(File::create(&cfg_path));
    try!(write!(f, "{}\n", encoded));

    println!("Wrote config {}: \n{}", cfg_path.display(), encoded);
    return Ok(cfg.clone());
}
