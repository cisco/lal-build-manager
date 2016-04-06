use std::io::{self, Error, ErrorKind};
use std::fs;
use std::path::Path;

// use util::globalroot::get_tarball_uri;
use util::artifactory::get_tarball_uri;
use Manifest;
use configure::Config;
use errors::{CliError, LalResult};

pub struct Component {
    pub name: String,
    pub version: u32,
    pub tarball: String,
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

fn fetch_component(cfg: Config, name: &str, version: Option<u32>) -> LalResult<Component> {
    use tar::Archive;
    use flate2::read::GzDecoder;
    use cache;

    let component = try!(get_tarball_uri(name, version));
    let tarname = ["./", name, ".tar"].concat();

    // always just download for now - TODO: eventually check cache
    let dl = download_to_path(&component.tarball, &tarname);
    if dl.is_ok() {
        debug!("Unpacking tarball {}", tarname);
        let data = try!(fs::File::open(&tarname));
        let decompressed = try!(GzDecoder::new(data)); // decoder reads data
        let mut archive = Archive::new(decompressed); // Archive reads decoded

        let extract_path = Path::new("./INPUT").join(&name);
        try!(fs::create_dir_all(&extract_path));
        try!(archive.unpack(&extract_path));

        // Move tarball into cfg.cache
        try!(cache::store_tarball(&cfg, name, component.version));
    }

    Ok(component)
}

fn clean_input() {
    let input = Path::new("./INPUT");
    if input.is_dir() {
        let _ = fs::remove_dir_all(&input).unwrap();
    }
}

pub fn install(manifest: Manifest,
               cfg: Config,
               xs: Vec<&str>,
               save: bool,
               savedev: bool)
               -> LalResult<()> {
    debug!("Install specific deps: {:?}", xs);

    let mut error = None;
    let mut installed = Vec::with_capacity(xs.len());
    for v in &xs {
        info!("Fetch {}", v);
        if v.contains("=") {
            let pair: Vec<&str> = v.split("=").collect();
            if let Ok(n) = pair[1].parse::<u32>() {
                match fetch_component(cfg.clone(), pair[0], Some(n)) {
                    Ok(c) => installed.push(c),
                    Err(e) => {
                        warn!("Failed to install {} ({})", pair[0], e);
                        error = Some(e);
                    }
                }
            } else {
                // TODO: this should try to install from stash!
                warn!("Failed to install {} labelled {} build from stash",
                      pair[1],
                      pair[0]);
                error = Some(CliError::InstallFailure);
            }
        } else {
            match fetch_component(cfg.clone(), &v, None) {
                Ok(c) => installed.push(c),
                Err(e) => {
                    warn!("Failed to install {} ({})", &v, e);
                    error = Some(e);
                }
            }
        }
    }
    if error.is_some() {
        return Err(error.unwrap());
    }

    // Update manifest if saving in any way
    if save || savedev {
        let mut mf = manifest.clone();
        // find reference to correct list
        let mut hmap = if save { mf.dependencies.clone() } else { mf.devDependencies.clone() };
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
        try!(mf.write());
    }
    Ok(())
}

pub fn uninstall(manifest: Manifest, xs: Vec<&str>, save: bool, savedev: bool) -> LalResult<()> {
    debug!("Removing dependencies {:?}", xs);

    // remove entries in xs from manifest.
    if save || savedev {
        let mut mf = manifest.clone();
        let mut hmap = if save { mf.dependencies.clone() } else { mf.devDependencies.clone() };
        for component in xs.clone() {
            // We could perhaps allow people to just specify ANY dependency
            // and have a generic save flag, which we could infer from
            // thus we could modify both maps if listing many components

            // This could work, but it's not currently what install does, so not doing it.
            // => all components uninstalled from either dependencies, or all from devDependencies
            // if doing multiple components from different maps, do multiple calls
            if ! hmap.contains_key(component) {
                return Err(CliError::MissingComponent(component.to_string()));
            }
            debug!("Removing {} from manifest", component);
            hmap.remove(component);
        }
        if save {
            mf.dependencies = hmap;
        } else {
            mf.devDependencies = hmap;
        }
        info!("Updating manifest with removed dependencies");
        try!(mf.write());
    }

    // delete the folder (ignore if the folder does not exist)
    let input = Path::new("./INPUT");
    if !input.is_dir() {
        return Ok(());
    }
    for component in xs {
        let pth = Path::new(&input).join(component);
        if pth.is_dir() {
            debug!("Deleting INPUT/{}", component);
            try!(fs::remove_dir_all(&pth));
        }
    }
    Ok(())
}

pub fn install_all(manifest: Manifest, cfg: Config, dev: bool) -> LalResult<()> {
    use std::thread;
    use std::sync::mpsc;

    debug!("Installing dependencies{}",
           if dev { " and devDependencies" } else { "" });
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
        info!("Fetch {} {}", k, v);
        let tx = tx.clone();
        let cfgcpy = cfg.clone();
        thread::spawn(move || {
            let r = fetch_component(cfgcpy, &k, Some(v)).map_err(|e| {
                warn!("Failed to completely install {} ({})", k, e);
                // likely symlinks inside tarball that are being dodgy
                // this is why we clean_input
            });
            tx.send(r.is_ok()).unwrap();
        });
    }

    // join
    let mut success = true;
    for _ in 0..len {
        let res = rx.recv().unwrap();
        success = res && success;
    }
    if !success {
        return Err(CliError::InstallFailure);
    }
    Ok(())
}
