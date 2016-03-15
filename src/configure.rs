use rustc_serialize::json;
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::io::prelude::*;
use errors::{CliError, LalResult};

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Config {
    pub artifactory: String,
    pub cache: String,
    pub target: String,
    pub container: String,
}

impl Config {
    pub fn new() -> LalResult<Config> {
        // unwrapping things that really must succeed here
        let home = env::home_dir().unwrap();
        let cachepath = Path::new(&home).join(".lal").join("cache");
        let cachedir = cachepath.as_path().to_str().unwrap();
        Ok(Config {
            artifactory: "http://engci-maven.cisco.com/artifactory/CME-group".to_string(),
            cache: cachedir.to_string(),
            target: "ncp.amd64".to_string(),
            container: "edonusdevelopers/centos_build".to_string(),
        })
    }
    pub fn read() -> LalResult<Config> {
        let home = env::home_dir().unwrap(); // crash if no $HOME
        let cfg_path = Path::new(&home).join(".lal/lalrc");
        if !cfg_path.exists() {
            return Err(CliError::MissingConfig);
        }
        let mut f = try!(fs::File::open(&cfg_path));
        let mut cfg_str = String::new();
        try!(f.read_to_string(&mut cfg_str));
        let res = try!(json::decode(&cfg_str));
        Ok(res)
    }
    pub fn write(&self) -> LalResult<()> {
        let home = env::home_dir().unwrap();
        let cfg_path = Path::new(&home).join(".lal").join("lalrc");

        let encoded = json::as_pretty_json(self);

        let mut f = try!(fs::File::create(&cfg_path));
        try!(write!(f, "{}\n", encoded));
        info!("Wrote config {}: \n{}", cfg_path.display(), encoded);
        Ok(())
    }
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

fn create_lal_dir() -> LalResult<PathBuf> {
    let home = env::home_dir().unwrap();
    let laldir = Path::new(&home).join(".lal");
    if !laldir.is_dir() {
        try!(fs::create_dir(&laldir));
    }
    Ok(laldir)
}
// TODO: need some extra sanity to also check that:
//   - docker is present and warn if not
//   - docker images contains cfg.container and provide info if not


pub fn configure(term_prompt: bool, save: bool) -> LalResult<Config> {
    let _ = try!(create_lal_dir());
    let mut cfg = try!(Config::new());

    if term_prompt {
        // Prompt for values:
        cfg.artifactory = prompt("artifactory", cfg.artifactory);
        cfg.cache = prompt("cache", cfg.cache);
        cfg.target = prompt("target", cfg.target);
        cfg.container = prompt("container", cfg.container);
    }
    if save {
        try!(cfg.write());
    }

    Ok(cfg.clone())
}
