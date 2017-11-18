#![allow(missing_docs)]

use std::fs;
use std::str::FromStr;
use std::vec::Vec;
use std::path::{Path, PathBuf};

use core::{CliError, LalResult, config_dir, ensure_dir_exists_fresh};


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LocalConfig {}

use super::{Backend, Component};

pub struct LocalBackend {
    /// Local config
    pub config: LocalConfig,
    /// Cache directory
    pub cache: String,
}

impl LocalBackend {
    pub fn new(cfg: &LocalConfig, cache: &str) -> Self {
        LocalBackend {
            config: cfg.clone(),
            cache: cache.into(),
        }
    }
}

/// Artifact backend trait for `LocalBackend`
///
/// This is intended to be used by the caching trait `CachedBackend`, but for
/// specific low-level use cases, these methods can be used directly.
impl Backend for LocalBackend {
    fn get_versions(&self, name: &str, loc: &str) -> LalResult<Vec<u32>> {

        let tar_dir = format!("cache/environments/{}/{}/", loc, name);
        let dentries = fs::read_dir(config_dir().join(tar_dir));
        let mut versions = vec![];
        for entry in dentries? {
            let path = entry?;
            if let Some(filename) = path.file_name().to_str() {
                if let Ok(version) = u32::from_str(filename) {
                    versions.push(version);
                }
            }
        }
        Ok(versions)
    }

    fn get_latest_version(&self, name: &str, loc: &str) -> LalResult<u32> {
        if let Some(&last) = self.get_versions(name, loc)?.last() {
            return Ok(last);
        }
        Err(CliError::BackendFailure("No versions found on local storage".into()))
    }

    fn get_component_info(
        &self,
        name: &str,
        version: Option<u32>,
        loc: &str,
    ) -> LalResult<Component> {
        info!("get_component_info: {} {:?} {}", name, version, loc);
        Ok(Component {
            name: name.into(),
            version: version.unwrap(),
            location: "".into(), // unused, component is already local
        })
    }

    fn publish_artifact(&self, name: &str, version: u32, env: &str) -> LalResult<()> {
        // this fn basically assumes all the sanity checks have been performed
        // files must exist and lockfile must be sensible
        let artifactdir = Path::new("./ARTIFACT");
        let tarball = artifactdir.join(format!("{}.tar.gz", name));
        let lockfile = artifactdir.join("lockfile.json");

        // prefix with environment
        let tar_dir = format!("cache/environments/{}/{}/{}/", env, name, version);
        let tar_path = format!("cache/environments/{}/{}/{}/{}.tar", env, name, version, name);
        let lock_path = format!("cache/environments/{}/{}/{}/lockfile.json", env, name, version);

        if let Some(full_tar_dir) = config_dir().join(tar_dir).to_str() {
            ensure_dir_exists_fresh(full_tar_dir)?;
        }

        fs::copy(tarball, config_dir().join(tar_path))?;
        fs::copy(lockfile, config_dir().join(lock_path))?;

        Ok(())
    }

    fn get_cache_dir(&self) -> String { self.cache.clone() }

    fn raw_fetch(&self, _url: &str, _dest: &PathBuf) -> LalResult<()> {
        unimplemented!()
    }
}
