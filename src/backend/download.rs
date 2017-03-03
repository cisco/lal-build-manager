use std::fs;
use std::path::{Path, PathBuf};

use backend::{Artifactory, Component};
use core::{CliError, LalResult};

fn is_cached(backend: &Artifactory, name: &str, version: u32, env: Option<&str>) -> bool {
    get_cache_dir(backend, name, version, env).is_dir()
}

fn get_cache_dir(backend: &Artifactory,
                     name: &str,
                     version: u32,
                     env: Option<&str>)
                     -> PathBuf {
    let pth = Path::new(&backend.cache);
    let leading_pth = match env {
        None => pth.join("globals"),
        Some(e) => pth.join("environments").join(e),
    };
    leading_pth.join(name).join(version.to_string())
}

fn store_tarball(backend: &Artifactory,
                     name: &str,
                     version: u32,
                     env: Option<&str>)
                     -> Result<(), CliError> {
    // 1. mkdir -p backend.cacheDir/$name/$version
    let destdir = get_cache_dir(backend, name, version, env);
    if !destdir.is_dir() {
        fs::create_dir_all(&destdir)?;
    }
    // 2. stuff $PWD/$name.tar in there
    let tarname = [name, ".tar"].concat();
    let dest = Path::new(&destdir).join(&tarname);
    let src = Path::new(".").join(&tarname);
    if !src.is_file() {
        return Err(CliError::MissingTarball);
    }
    debug!("Move {:?} -> {:?}", src, dest);
    fs::copy(&src, &dest)?;
    fs::remove_file(&src)?;

    Ok(())
}

fn download_to_path(url: &str, save: &PathBuf) -> LalResult<()> {
    use hyper::{self, Client};
    use std::io::prelude::{Write, Read};

    debug!("GET {}", url);
    let client = Client::new();
    let mut res = client.get(url).send()?;
    if res.status != hyper::Ok {
        return Err(CliError::ArtifactoryFailure(format!("GET request with {}", res.status)));
    }

    let mut buffer: Vec<u8> = Vec::new();
    res.read_to_end(&mut buffer)?;
    let mut f = fs::File::create(save)?;
    f.write_all(&buffer)?;
    Ok(())
}

// helper for fetch_and_unpack_component and fetch_from_stash
fn extract_tarball_to_input(tarname: PathBuf, component: &str) -> LalResult<()> {
    use tar::Archive;
    use flate2::read::GzDecoder;

    let data = fs::File::open(tarname)?;
    let decompressed = GzDecoder::new(data)?; // decoder reads data
    let mut archive = Archive::new(decompressed); // Archive reads decoded

    let extract_path = Path::new("./INPUT").join(component);
    let _ = fs::remove_dir_all(&extract_path); // remove current dir if exists
    fs::create_dir_all(&extract_path)?;
    archive.unpack(&extract_path)?;
    Ok(())
}

/// helper for `install::update`
pub fn fetch_from_stash(backend: &Artifactory, component: &str, stashname: &str) -> LalResult<()> {
    let tarname = get_path_to_stashed_component(backend, component, stashname)?;
    extract_tarball_to_input(tarname, component)?;
    Ok(())
}


/// helper for `install::export`
pub fn get_path_to_stashed_component(backend: &Artifactory,
                                     component: &str,
                                     stashname: &str)
                                     -> LalResult<PathBuf> {
    let stashdir = Path::new(&backend.cache).join("stash").join(component).join(stashname);
    if !stashdir.is_dir() {
        return Err(CliError::MissingStashArtifact(format!("{}/{}", component, stashname)));
    }
    debug!("Inferring stashed version {} of component {}",
           stashname,
           component);
    let tarname = stashdir.join(format!("{}.tar.gz", component));
    Ok(tarname)
}

/// Download an artifact into stash and return its path and details
pub fn fetch_via_artifactory(backend: &Artifactory,
                         name: &str,
                         version: Option<u32>,
                         env: Option<&str>)
                         -> LalResult<(PathBuf, Component)> {

    use backend::Backend;
    trace!("Locate component {}", name);

    let component = backend.get_tarball_url(name, version, env)?;

    if !is_cached(backend, &component.name, component.version, env) {
        // download to PWD then move it to stash immediately
        let local_tarball = Path::new(".").join(format!("{}.tar", name));
        download_to_path(&component.tarball, &local_tarball)?;
        store_tarball(backend, name, component.version, env)?;
    }
    assert!(is_cached(backend, &component.name, component.version, env),
            "cached component");

    trace!("Fetching {} from cache", name);
    let tarname = get_cache_dir(backend, &component.name, component.version, env)
        .join(format!("{}.tar", name));
    Ok((tarname, component))
}

/// Full fetch + unpack procedure used by fetch subcommand for non stashed comps
pub fn fetch_and_unpack_component(backend: &Artifactory,
                              name: &str,
                              version: Option<u32>,
                              env: Option<&str>)
                              -> LalResult<Component> {
    let (tarname, component) = fetch_via_artifactory(backend, name, version, env)?;

    debug!("Unpacking tarball {} for {}",
           tarname.to_str().unwrap(),
           component.name);
    extract_tarball_to_input(tarname, name)?;

    Ok(component)
}
