use rustc_serialize::json;
use std::path::Path;
use std::fs;
use std::env;
use std::io::prelude::*;
use errors::CliError;

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Config {
    pub registry: String,
    pub cache: String,
    pub target: String,
    pub container: String,
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
    default
}

pub fn current_config() -> Result<Config, CliError> {
    let home = env::home_dir().unwrap(); // crash if no $HOME
    let cfg_path = Path::new(&home).join(".lal/lalrc");
    if !cfg_path.exists() {
        return Err(CliError::MissingConfig);
    }
    let mut f = try!(fs::File::open(&cfg_path));
    let mut cfg_str = String::new();
    try!(f.read_to_string(&mut cfg_str));
    // TODO: handle last error too
    let res = try!(json::decode(&cfg_str));
    Ok(res)
}

pub fn configure(term_prompt: bool) -> Result<Config, CliError> {
    let mut cfg = Config {
        registry: "http://localhost".to_string(),
        cache: "~/.lal/cache".to_string(),
        target: "ncp.amd64".to_string(),
        container: "edonusdevelopers/centos_build".to_string(),
    };

    let home = env::home_dir().unwrap(); // crash if no $HOME
    let cfg_path = Path::new(&home).join(".lal/lalrc");
    let laldir = Path::new(&home).join(".lal");
    if !laldir.is_dir() {
        try!(fs::create_dir(&laldir));
    }

    if term_prompt {
        // Prompt for values:
        cfg.registry = prompt("registry", cfg.registry);
        cfg.cache = prompt("cache", cfg.cache);
        cfg.target = prompt("target", cfg.target);
        cfg.container = prompt("container", cfg.container);
    }

    // Encode
    let encoded = json::as_pretty_json(&cfg);

    let mut f = try!(fs::File::create(&cfg_path));
    try!(write!(f, "{}\n", encoded));

    info!("Wrote config {}: \n{}", cfg_path.display(), encoded);

    // TODO: check that docker is present and warn if not
    // TODO: check that docker images contains cfg.container and provide info if not
    Ok(cfg.clone())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::{Path, PathBuf};
    use std::fs;

    use configure;

    fn lal_dir() -> PathBuf {
        let home = env::home_dir().unwrap();
        Path::new(&home).join(".lal/")
    }
    // These tests screw with the other tests which are also reading lalrc
    // Can run them from scratch with `cargo test -- --ignored`

    #[test]
    #[ignore]
    fn hide_lalrc() {
        let ldir = lal_dir();
        if ldir.is_dir() {
            fs::remove_dir_all(&ldir).unwrap();
        }
        assert_eq!(ldir.is_dir(), false);
    }

    #[test]
    #[ignore]
    fn configure_without_lalrc() {
        let r = configure::configure(false);
        assert_eq!(r.is_ok(), true);
        let cfg = configure::current_config();
        assert_eq!(cfg.is_ok(), true);
    }
}
