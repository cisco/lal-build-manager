use std::fs;
use std::path::{Path, PathBuf};

use crate::channel::Channel;
use crate::core::{output, CliError, LalResult};
use crate::storage::{Backend, CachedBackend, Component};

fn is_cached<T: Backend + ?Sized>(
    backend: &T,
    name: &str,
    version: u32,
    env: &str,
    channel: &Channel,
) -> bool {
    get_cache_dir(backend, name, version, env, channel).is_dir()
}

fn get_cache_dir<T: Backend + ?Sized>(
    backend: &T,
    name: &str,
    version: u32,
    env: &str,
    channel: &Channel,
) -> PathBuf {
    let cache = backend.get_cache_dir();
    Path::new(&cache)
        .join(channel.to_path())
        .join("environments")
        .join(env)
        .join(name)
        .join(version.to_string())
}

fn store_tarball<T: Backend + ?Sized>(
    backend: &T,
    name: &str,
    version: u32,
    env: &str,
    channel: &Channel,
) -> Result<(), CliError> {
    // 1. mkdir -p cacheDir/$name/$version
    let destdir = get_cache_dir(backend, name, version, env, channel);
    if !destdir.is_dir() {
        fs::create_dir_all(&destdir)?;
    }
    // 2. stuff $PWD/$name.tar.gz in there
    let tarname = [name, ".tar.gz"].concat();
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

// helper for the unpack_ functions
fn extract_tarball_to_input(tarname: PathBuf, component: &str) -> LalResult<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let extract_path = Path::new("./INPUT").join(component);
    let _ = fs::remove_dir_all(&extract_path); // remove current dir if exists
    fs::create_dir_all(&extract_path)?;

    // Open file, conditionally wrap a progress bar around the file reading
    if cfg!(feature = "progress") {
        #[cfg(feature = "progress")]
        {
            use super::progress::ProgressReader;
            let data = fs::File::open(tarname)?;
            let progdata = ProgressReader::new(data)?;
            let decompressed = GzDecoder::new(progdata)?; // decoder reads data (proxied)
            let mut archive = Archive::new(decompressed); // Archive reads decoded
            archive.unpack(&extract_path)?;
        }
    } else {
        let data = fs::File::open(tarname)?;
        let decompressed = GzDecoder::new(data)?; // decoder reads data
        let mut archive = Archive::new(decompressed); // Archive reads decoded
        archive.unpack(&extract_path)?;
    };

    Ok(())
}

/// Cacheable trait implemented for all Backends.
///
/// As long as we have the Backend trait implemented, we can add a caching layer
/// around this, which implements the basic compression ops and file gymnastics.
///
/// Most subcommands should be OK with just using this trait rather than using
/// `Backend` directly as this does the stuff you normally would want done.
impl<T: ?Sized> CachedBackend for T
where
    T: Backend,
{
    /// Get the latest versions of a component across all supported environments
    ///
    /// Because the versions have to be available in all environments, these numbers may
    /// not contain the highest numbers available on specific environments.
    fn get_latest_supported_versions(
        &self,
        name: &str,
        environments: Vec<String>,
        channel: &Channel,
    ) -> LalResult<Vec<u32>> {
        use std::collections::BTreeSet;
        let mut result = BTreeSet::new();
        let mut first_pass = true;
        for e in environments {
            let eres: BTreeSet<_> = self
                .get_versions(name, &e, channel)?
                .into_iter()
                .take(100)
                .collect();
            info!("Last versions for {} in {} env is {:?}", name, e, eres);
            if first_pass {
                // if first pass, can't take intersection with something empty, start with first result
                result = eres;
                first_pass = false;
            } else {
                result = result.clone().intersection(&eres).cloned().collect();
            }
        }
        debug!("Intersection of allowed versions {:?}", result);
        Ok(result.into_iter().collect())
    }

    /// Locate a proper component, downloading it and caching if necessary
    fn retrieve_published_component(
        &self,
        name: &str,
        version: Option<u32>,
        env: &str,
        channel: &Channel,
    ) -> LalResult<(PathBuf, Component)> {
        trace!("Locate component {}", name);

        let component = self.get_component_info(name, version, env, channel)?;

        if !is_cached(self, &component.name, component.version, env, channel) {
            self.cache_published_component(&component, env)?;
        }
        assert!(
            is_cached(self, &component.name, component.version, env, channel),
            "cached component"
        );

        trace!("Fetching {} from cache", name);
        let base_name =
            get_cache_dir(self, &component.name, component.version, env, channel).join(name);

        // Older versions of lal may have stored the artefacts without the gz extension. Check for this and fix it.
        // This is just a rename as the files were gzipped, however this was not reflected in their name.
        let compressed_name = base_name.with_extension("tar.gz");
        let uncompressed_name = base_name.with_extension("tar");
        if uncompressed_name.exists() {
            assert!(!compressed_name.exists());
            fs::rename(uncompressed_name, &compressed_name)?;
        }

        if !(compressed_name.exists()) {
            // The cache has somehow become corrupted, try to update it.
            self.cache_published_component(&component, env)?;
            assert!(compressed_name.exists());
        }

        Ok((compressed_name, component))
    }

    fn cache_published_component(&self, component: &Component, env: &str) -> LalResult<()> {
        // Download to PWD then move it to stash immediately
        let local_tarball = Path::new(".").join(format!("{}.tar.gz", component.name));
        self.raw_fetch(&component.location, &local_tarball)?;
        store_tarball(
            self,
            &component.name,
            component.version,
            env,
            &component.channel,
        )
    }

    // basic functionality for `fetch`/`update`
    fn unpack_published_component(
        &self,
        name: &str,
        version: Option<u32>,
        env: &str,
        channel: &Channel,
    ) -> LalResult<Component> {
        let (tarname, component) =
            self.retrieve_published_component(name, version, env, channel)?;

        debug!(
            "Unpacking tarball {} for {}",
            tarname.to_str().unwrap(),
            component.name
        );
        extract_tarball_to_input(tarname, name)?;

        Ok(component)
    }

    /// helper for `update`
    fn unpack_stashed_component(&self, name: &str, code: &str) -> LalResult<()> {
        let tarpath = self.retrieve_stashed_component(name, code)?;

        extract_tarball_to_input(tarpath, name)?;
        Ok(())
    }

    /// helper for unpack_, `export`
    fn retrieve_stashed_component(&self, name: &str, code: &str) -> LalResult<PathBuf> {
        let tarpath = Path::new(&self.get_cache_dir())
            .join("stash")
            .join(name)
            .join(code)
            .join(format!("{}.tar.gz", name));
        if !tarpath.is_file() {
            debug!("Could not find {:?}", tarpath);
            return Err(CliError::MissingStashArtifact(format!("{}/{}", name, code)));
        }
        Ok(tarpath)
    }

    // helper for `stash`
    fn stash_output(&self, name: &str, code: &str) -> LalResult<()> {
        let destdir = Path::new(&self.get_cache_dir())
            .join("stash")
            .join(name)
            .join(code);
        debug!("Creating {:?}", destdir);
        fs::create_dir_all(&destdir)?;

        // Tar it straight into destination
        output::tar(&destdir.join(format!("{}.tar.gz", name)))?;

        // Copy the lockfile there for users inspecting the stashed folder
        // NB: this is not really needed, as it's included in the tarball anyway
        fs::copy("./OUTPUT/lockfile.json", destdir.join("lockfile.json"))?;
        Ok(())
    }
}
