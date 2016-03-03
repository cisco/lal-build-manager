use std::io;
use regex::Regex;

struct Component {
    name: String,
    version: u32,
    tarball: String, // TODO: Option<Path> for cache path
}

fn get_latest(uri: &str) -> Option<u32> {
    use curl::http;

    println!("GET {}", uri);
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

        // println!("version: {}", version);
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

fn get_blob(uri: &str) -> io::Result<String> {
    use curl::http;
    use std::io::{Error, ErrorKind};

    println!("GET {}", uri);
    let resp = http::handle().get(uri).exec().unwrap();

    if resp.get_code() == 200 {
        let body = String::from_utf8_lossy(resp.get_body());
        // println!("resp {}", body);

        // Assume yaml is sane for now as this is a temporary hack:
        // Since yaml is a temporary interface, this eludes the need for a yaml parser
        let re = Regex::new(r"blob: (.{64})").unwrap();
        if re.is_match(&body) {
            let blob = re.captures(&body).unwrap().at(1).unwrap().to_string();
            // println!("blob: {}", blob);

            // split the urls into chunks of 4
            let mut splits = vec![];
            for i in 0..16 {
                splits.push(&blob[4 * i..4 * (i + 1)]);
            }
            return Ok(splits.join("/"));
        }
    }
    return Err(Error::new(ErrorKind::Other, "no yaml found"));
}

fn get_dependency_url_latest(name: &str) -> io::Result<Component> {
    use std::io::{Error, ErrorKind};

    let globalroot = "http://builds.lal.cisco.com/globalroot/ARTIFACTS";
    let target = "ncp.amd64"; // TODO: from config::Config

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
        println!("Found latest version as {}", v);
        get_dependency_url(name, v).map(|uri| {
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

fn get_dependency_url(name: &str, version: u32) -> io::Result<String> {
    use std::io::{Error, ErrorKind};

    let globalroot = "http://builds.lal.cisco.com/globalroot/ARTIFACTS";
    let target = "ncp.amd64"; // TODO: from config::Config

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

    if let Ok(blob) = get_blob(&cloud_yurl) {
        println!("Found corresponding blob in cloud");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else if let Ok(blob) = get_blob(&def_yurl) {
        println!("Found corresponding blob in default");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else {
        Err(Error::new(ErrorKind::Other, "failed to find blob"))
    }
}

fn get_tarball_uri(name: &str, version: Option<u32>) -> io::Result<Component> {
    if let Some(v) = version {
        get_dependency_url(name, v).map(|uri| {
            Component {
                tarball: uri,
                version: v,
                name: name.to_string(),
            }
        })
    } else {
        get_dependency_url_latest(name)
    }
}

fn fetch_component(name: &str, version: Option<u32>) -> io::Result<Component> {
    use tar::Archive;
    use flate2::read::GzDecoder;
    use std::fs;
    use std::path::Path;
    use std::env;

    let component = try!(get_tarball_uri(name, version));
    // println!("fetching dependency {} at {}", component.version.to_string(), component.tarball);

    let tarname = ["./", name, ".tar"].concat();

    let dl = download_to_path(&component.tarball, &tarname);
    if dl.is_ok() {
        let data = try!(fs::File::open(&tarname));
        let decompressed = try!(GzDecoder::new(data));
        let mut archive = Archive::new(decompressed);

        let pwd = env::current_dir().unwrap();
        let extract_path = Path::new(&pwd).join("INPUT").join(&name);
        try!(fs::create_dir_all(&extract_path));
        try!(archive.unpack(&extract_path));
        // TODO: move tarball in PWD to cachedir from lalrc
    }

    Ok(component)
}

pub fn install(xs: Vec<&str>, save: bool, savedev: bool) {
    use init;
    println!("Install specific deps: {:?} {} {}", xs, save, savedev);

    let mut installed = Vec::with_capacity(xs.len());
    for v in &xs {
        if v.contains("=") {
            let pair: Vec<&str> = v.split("=").collect();
            if let Ok(n) = pair[1].parse::<u32>() {
                match fetch_component(pair[0], Some(n)) {
                    Ok(c) => installed.push(c),
                    Err(e) => println!("Failed to install {} ({})", pair[0], e),
                }
            } else {
                println!("Ignoring {} due to invalid version number", pair[0]);
            }
        } else {
            match fetch_component(&v, None) {
                Ok(c) => installed.push(c),
                Err(e) => println!("Failed to install {} ({})", &v, e),
            }
        }
    }

    // Update manifest if saving in any way
    if save || savedev {
        let mut mf = init::read_manifest().unwrap();
        // find reference to correct list
        let mut hmap = if save {
            mf.dependencies.clone()
        } else {
            mf.devDependencies.clone()
        };
        for c in &installed {
            // println!("Successfully installed {} at version {}", &c.name, c.version);
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

fn download_to_path(uri: &str, save: &str) -> io::Result<bool> {
    use std::fs::File;
    use std::path::Path;
    use curl::http;
    use std::io::prelude::*;

    println!("GET {}", uri);
    let resp = http::handle().get(uri).exec().unwrap();

    if resp.get_code() == 200 {
        let r = resp.get_body();
        let path = Path::new(save);
        let mut f = try!(File::create(&path));
        try!(f.write_all(r));
    }
    Ok(resp.get_code() == 200)
}

pub fn install_all(dev: bool) {
    use init;
    use std::thread;
    use std::sync::mpsc;

    println!("Installing all dependencies{}",
             if dev {
                 " and devDependencies"
             } else {
                 ""
             });
    let manifest = init::read_manifest().unwrap();

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
        println!("Installing {} {}", k, v);
        let tx = tx.clone();
        thread::spawn(move || {
            let _ = fetch_component(&k, Some(v)).map_err(|e| {
                println!("Failed to install {} ({})", &v, e);
            });
            tx.send(()).unwrap();
        });
    }
    // join
    for _ in 0..len {
        rx.recv().unwrap();
    }
}
