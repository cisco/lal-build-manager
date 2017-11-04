#![allow(missing_docs)]
#![allow(unused_variables)]
#![allow(unused_imports)]
use std::vec::Vec;

use std::fs::File;
use std::path::{Path, PathBuf};


use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use hubcaps::{Credentials, Github};

use hubcaps::releases::{Release, Releases, ReleaseOptions, ReleaseOptionsBuilder};

use core::{CliError, LalResult};


/// Github credentials
#[derive(Serialize, Deserialize, Clone)]
pub struct GithubCredentials {
    /// Personal access token with upload access
    pub token: String,
}

/// Static Github locations
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GithubConfig {
    /// Github organisation
    pub organisation: String,
    /// Optional upload credentials
    pub credentials: Option<GithubCredentials>,
}

use super::{Backend, Component};

/// Everything we need for Github to implement the Backend trait
pub struct GithubBackend {
    /// Github config and credentials
    pub config: GithubConfig,
    /// Cache directory
    pub cache: String,
    /// Github client with a tls configured hyper client
    pub client: Github,
}

impl GithubBackend {
    pub fn new(cfg: &GithubConfig, cache: &str) -> Self {
        let creds = if let Some(c) = cfg.credentials.clone() {
            Credentials::Token(c.token)
        } else {
            Credentials::default()
        };
        let github = Github::new(
            format!("lal/{}", env!("CARGO_PKG_VERSION")),
            Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap())),
            creds
        );

        GithubBackend {
            config: cfg.clone(),
            client: github,
            cache: cache.into(),
        }
    }
}


/// Artifact backend trait for `GithubBackend`
///
/// This is intended to be used by the caching trait `CachedBackend`, but for
/// specific low-level use cases, these methods can be used directly.
impl Backend for GithubBackend {
    fn get_versions(&self, name: &str, loc: &str) -> LalResult<Vec<u32>> {
        unimplemented!();
    }

    fn get_latest_version(&self, name: &str, loc: &str) -> LalResult<u32> {
        unimplemented!();
    }

    fn get_component_info(
        &self,
        name: &str,
        version: Option<u32>,
        loc: &str,
    ) -> LalResult<Component> {
        unimplemented!();
    }

    fn publish_artifact(&self, name: &str, version: u32, env: &str) -> LalResult<()> {
        // this fn basically assumes all the sanity checks have been performed
        // files must exist and lockfile must be sensible
        let artdir = Path::new("./ARTIFACT");
        let tarball = artdir.join(format!("{}.tar.gz", name));
        let lockfile = artdir.join("lockfile.json");


        // 1. create a release
        // TODO: needs sha from lockfile?
        let res = Releases::new(&self.client, self.config.organisation.clone(), name);
        let opts = ReleaseOptionsBuilder::new(version.to_string()).build();
        let release : Release = res.create(&opts)?;

        // 2. create an asset on this release
        // TODO: this part of the api is missing from hubcaps

        // uri prefix if specific env upload
        //let prefix = format!("env/{}/", env);
        //let tar_uri = format!("{}{}/{}/{}.tar.gz", prefix, name, version, name);
        //let mut tarf = File::open(tarball)?;
        //upload_artifact(&self.config, &tar_uri, &mut tarf)?;

        //let mut lockf = File::open(lockfile)?;
        //let lf_uri = format!("{}{}/{}/lockfile.json", prefix, name, version);
        //upload_artifact(&self.config, &lf_uri, &mut lockf)?;
        unimplemented!();
    }

    fn get_cache_dir(&self) -> String { self.cache.clone() }

    fn raw_fetch(&self, url: &str, dest: &PathBuf) -> LalResult<()> {
        unimplemented!();
    }
}
