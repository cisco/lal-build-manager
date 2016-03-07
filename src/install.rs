use std::io;
use std::io::{Error, ErrorKind};
use std::fs;
use std::path::Path;
use std::env;
use regex::Regex;
use init::Manifest;
use configure::Config;

struct Component {
    name: String,
    version: u32,
    tarball: String, // TODO: Option<Path> for cache path
}

fn get_latest(uri: &str) -> Option<u32> {
    use curl::http;

    debug!("GET {}", uri);
    let resp = http::handle().get(uri).exec().unwrap();

    if resp.get_code() == 200 {
        let body = String::from_utf8_lossy(resp.get_body());

        // Assume yaml is sane for now as this is a temporary hack:
        // Since yaml is a temporary interface, this eludes the need for a yaml parser
        let re = Regex::new(r"version: '([^']+)'").unwrap();
        if !re.is_match(&body) {
            return None;
        }
        let matches = re.captures(&body).unwrap();
        let version = matches.at(1).unwrap().to_string();

        debug!("Parsed version: {} from {}", version, uri);
        if version == "latest" {
            return None;
        }
        // otherwise version is an int
        if let Ok(n) = version.parse::<u32>() {
            return Some(n);
        }
    }
    None
}

fn get_blob(uri: &str) -> Option<String> {
    use curl::http;

    debug!("GET {}", uri);
    let resp = http::handle().get(uri).exec().unwrap();

    if resp.get_code() == 200 {
        let body = String::from_utf8_lossy(resp.get_body());
        // trace!("resp {}", body);

        // Assume yaml is sane for now as this is a temporary hack:
        // Since yaml is a temporary interface, this eludes the need for a yaml parser
        let re = Regex::new(r"blob: (.{64})").unwrap();
        if re.is_match(&body) {
            let blob = re.captures(&body).unwrap().at(1).unwrap().to_string();
            debug!("Found blob: {}", blob);

            // split the urls into chunks of 4
            let mut splits = vec![];
            for i in 0..16 {
                splits.push(&blob[4 * i..4 * (i + 1)]);
            }
            return Some(splits.join("/"));
        }
    }
    None
}

fn get_dependency_url_latest(name: &str, target: &str) -> io::Result<Component> {
    let globalroot = "http://builds.lal.cisco.com/globalroot/ARTIFACTS";

    // try cloud first
    let mut cloud_url = [globalroot, name, target, "global", "cloud", "latest"].join("/");
    cloud_url.push_str(".yaml");
    let mut default_url = [globalroot, name, target, "global", "default", "latest"].join("/");
    default_url.push_str(".yaml");

    let cloud_version = get_latest(&cloud_url);
    let default_version = get_latest(&default_url);

    // Checking cloud yaml first, then default
    if cloud_version.is_some() || default_version.is_some() {
        let v = if cloud_version.is_some() {
            cloud_version.unwrap()
        } else {
            default_version.unwrap()
        };
        debug!("Found latest version as {}", v);
        get_dependency_url(name, target.as_ref(), v).map(|uri| {
            Component {
                tarball: uri,
                version: v,
                name: name.to_string(),
            }
        })
    } else {
        Err(Error::new(ErrorKind::Other, "failed to find component"))
    }
}

fn get_dependency_url(name: &str, target: &str, version: u32) -> io::Result<String> {
    let globalroot = "http://builds.lal.cisco.com/globalroot/ARTIFACTS";

    let mut cloud_yurl = [globalroot, name, target, "global", "cloud"].join("/");
    cloud_yurl.push_str("/");
    cloud_yurl.push_str(&version.to_string());
    cloud_yurl.push_str(".yaml");

    let mut def_yurl = [globalroot, name, target, "global", "default"].join("/");
    def_yurl.push_str("/");
    def_yurl.push_str(&version.to_string());
    def_yurl.push_str(".yaml");

    let mut tar_url = [globalroot, ".blobs"].join("/");
    tar_url.push_str("/");

    if let Some(blob) = get_blob(&cloud_yurl) {
        debug!("Found corresponding blob in cloud");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else if let Some(blob) = get_blob(&def_yurl) {
        debug!("Found corresponding blob in default");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else {
        Err(Error::new(ErrorKind::Other, "failed to find blob"))
    }
}

fn get_tarball_uri(name: &str, target: &str, version: Option<u32>) -> io::Result<Component> {
    if let Some(v) = version {
        get_dependency_url(name, target, v).map(|uri| {
            Component {
                tarball: uri,
                version: v,
                name: name.to_string(),
            }
        })
    } else {
        get_dependency_url_latest(name, target)
    }
}

fn download_to_path(uri: &str, save: &str) -> io::Result<()> {
    use curl::http;
    use std::io::prelude::*;

    debug!("GET {}", uri);
    let resp = http::handle().get(uri).exec().unwrap();

    if resp.get_code() == 200 {
        let r = resp.get_body();
        let path = Path::new(save);
        let mut f = try!(fs::File::create(&path));
        try!(f.write_all(r));
        Ok(())
    } else {
        Err(Error::new(ErrorKind::Other, "failed to download file"))
    }
}

fn fetch_component(cfg: Config, name: &str, version: Option<u32>) -> io::Result<Component> {
    use tar::Archive;
    use flate2::read::GzDecoder;
    use cache;

    let component = try!(get_tarball_uri(name, cfg.target.as_ref(), version));
    let tarname = ["./", name, ".tar"].concat();

    // always just download for now - TODO: eventually check cache
    let dl = download_to_path(&component.tarball, &tarname);
    if dl.is_ok() {
        debug!("Unpacking tarball {}", tarname);
        let data = try!(fs::File::open(&tarname));
        let decompressed = try!(GzDecoder::new(data));
        let mut archive = Archive::new(decompressed);

        let pwd = env::current_dir().unwrap();
        let extract_path = Path::new(&pwd).join("INPUT").join(&name).join(&cfg.target);
        try!(fs::create_dir_all(&extract_path));
        try!(archive.unpack(&extract_path));

        // Move tarball into cfg.cache
        let r = cache::store_tarball(&cfg, name, component.version);
        if let Err(e) = r {
            // TODO: wrap this in CliError later
            error!("Failed to cache {}: {}", name, e);
            return Err(Error::new(ErrorKind::Other, "failed to cache component"));
        }
    }

    Ok(component)
}

fn clean_input() {
    let input = Path::new(&env::current_dir().unwrap()).join("INPUT");
    if input.is_dir() {
        let _ = fs::remove_dir_all(&input).unwrap();
    }
}

pub fn install(manifest: Manifest, cfg: Config, xs: Vec<&str>, save: bool, savedev: bool) {
    use init;
    info!("Install specific deps: {:?}", xs);

    let mut installed = Vec::with_capacity(xs.len());
    for v in &xs {
        if v.contains("=") {
            let pair: Vec<&str> = v.split("=").collect();
            if let Ok(n) = pair[1].parse::<u32>() {
                match fetch_component(cfg.clone(), pair[0], Some(n)) {
                    Ok(c) => installed.push(c),
                    Err(e) => warn!("Failed to install {} ({})", pair[0], e),
                }
            } else {
                // TODO: this should try to install from stash!
                warn!("Ignoring {} due to invalid version number", pair[0]);
            }
        } else {
            match fetch_component(cfg.clone(), &v, None) {
                Ok(c) => installed.push(c),
                Err(e) => warn!("Failed to install {} ({})", &v, e),
            }
        }
    }

    // Update manifest if saving in any way
    if save || savedev {
        let mut mf = manifest.clone();
        // find reference to correct list
        let mut hmap = if save {
            mf.dependencies.clone()
        } else {
            mf.devDependencies.clone()
        };
        for c in &installed {
            debug!("Successfully installed {} at version {}",
                   &c.name,
                   c.version);
            if hmap.contains_key(&c.name) {
                *hmap.get_mut(&c.name).unwrap() = c.version;
            } else {
                hmap.insert(c.name.clone(), c.version);
            }
        }
        if save {
            mf.dependencies = hmap;
        } else {
            mf.devDependencies = hmap;
        }
        let _ = init::save_manifest(&mf);
    }
}

// pub fn uninstall(manifest: Manifest, xs: Vec<&str>, save: bool, savedev: bool) {
//    // TODO: implement
// }

pub fn install_all(manifest: Manifest, cfg: Config, dev: bool) {
    use std::thread;
    use std::sync::mpsc;

    info!("Installing all dependencies{}",
          if dev {
              " and devDependencies"
          } else {
              ""
          });
    clean_input();

    // create the joined hashmap of dependencies and possibly devdependencies
    let mut deps = manifest.dependencies.clone();
    if dev {
        for (k, v) in &manifest.devDependencies {
            deps.insert(k.clone(), v.clone());
        }
    }
    let len = deps.len();

    // install them in parallel
    let (tx, rx) = mpsc::channel();
    for (k, v) in deps {
        info!("Installing {} {}", k, v);
        let tx = tx.clone();
        let cfgcpy = cfg.clone();
        thread::spawn(move || {
            let _ = fetch_component(cfgcpy, &k, Some(v)).map_err(|e| {
                warn!("Failed to install {} ({})", &v, e);
            });
            tx.send(()).unwrap();
        });
    }
    // join
    for _ in 0..len {
        rx.recv().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::env;
    use std::fs;

    use install::install;
    use init;
    use configure;

    fn component_dir(name: &str) -> PathBuf {
        Path::new(&env::current_dir().unwrap()).join("INPUT").join(&name).join("ncp.amd64")
    }

    #[test]
    #[ignore]
    fn blank_state() {
        let input = Path::new(&env::current_dir().unwrap()).join("INPUT");
        if input.is_dir() {
            fs::remove_dir_all(&input).unwrap();
        }
        assert_eq!(input.is_dir(), false);
    }

    #[test]
    fn install_basic() {
        let manifest = init::read_manifest();
        assert_eq!(manifest.is_ok(), true);
        let mf = manifest.unwrap();
        let config = configure::current_config();
        assert_eq!(config.is_ok(), true);
        let cfg = config.unwrap();

        install(mf.clone(), cfg.clone(), vec!["gtest"], false, false);
        assert_eq!(component_dir("gtest").is_dir(), true);
        install(mf.clone(), cfg.clone(), vec!["libyaml"], false, false);
        assert_eq!(component_dir("libyaml").is_dir(), true);
    }
}
