use std::io;
use regex::Regex;

struct Component {
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

    if let Some(v) = get_latest(&cloud_url) {
        println!("Found latest version from cloud as {}", v);
        match get_dependency_url(name, v) {
            Ok(uri) => {
                Ok(Component {
                    tarball: uri,
                    version: v,
                })
            }
            Err(e) => Err(e),
        }
    } else if let Some(v) = get_latest(&default_url) {
        println!("Found latest version from default as {}", v);
        match get_dependency_url(name, v) {
            Ok(uri) => {
                Ok(Component {
                    tarball: uri,
                    version: v,
                })
            }
            Err(e) => Err(e),
        }
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

    if let Ok(blob) = get_blob(&cloud_yurl) {
        println!("Found corresponding blob in cloud");
        let mut tar_url = [globalroot, ".blobs"].join("/");
        tar_url.push_str("/");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else if let Ok(blob) = get_blob(&def_yurl) {
        println!("Found corresponding blob in default");
        let mut tar_url = [globalroot, ".blobs"].join("/");
        tar_url.push_str("/");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else {
        Err(Error::new(ErrorKind::Other, "failed to find blob"))
    }
}

fn get_tarball_uri(name: &str, version: Option<u32>) -> io::Result<Component> {
    if let Some(v) = version {
        match get_dependency_url(name, v) {
            Ok(uri) => {
                Ok(Component {
                    tarball: uri,
                    version: v,
                })
            }
            Err(s) => Err(s),
        }
    } else {
        match get_dependency_url_latest(name) {
            Ok(res) => Ok(res),
            Err(s) => Err(s),
        }
    }
}

fn fetch_component(name: &str, version: Option<u32>) -> io::Result<Component> {
    let component = try!(get_tarball_uri(name, version));
    // println!("fetching dependency {} at {}", component.version.to_string(), component.tarball);

    let mut tarname = name.to_string();
    tarname.push_str(".tar.gz");
    let _ = download_to_path(&component.tarball, &tarname);
    Ok(component)
}

pub fn install(xs: Vec<&str>, save: bool, savedev: bool) {
    println!("Install specific deps: {:?} {} {}", xs, save, savedev);
    for v in &xs {
        if v.contains("=") {
            let pair: Vec<&str> = v.split("=").collect();
            if let Ok(n) = pair[1].parse::<u32>() {
                let _ = fetch_component(pair[0], Some(n));
            }
            else {
                println!("Ignoring {} due to invalid version number", pair[0]);
            }
        } else {
            let _ = fetch_component(&v, None);
        }
    }
    // TODO: use returns of fetch_component to implement save and savedev.
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

    let (tx, rx) = mpsc::channel();
    for (k, v) in deps {
        println!("Installing {} {}", k, v);
        let tx = tx.clone();
        thread::spawn(move || {
            let _ = fetch_component(&k, Some(v));
            tx.send(()).unwrap();
        });
    }
    // join
    for _ in 0..len {
        rx.recv().unwrap();
    }
}
