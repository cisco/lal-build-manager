#![allow(missing_docs)]

use std::vec::Vec;
use std::io::Read;
use std::fs::File;

use serde_json;
use semver::Version;
use sha1;
use hyper::{self, Client};
use hyper::header::{Authorization, Basic};
use hyper::status::StatusCode;

use core::{CliError, LalResult};


/// Artifactory credentials
#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    /// Upload username
    pub username: String,
    /// Upload password
    pub password: String,
}

/// Static Artifactory locations
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArtifactoryConfig {
    /// Location of artifactory API master (for API queries)
    pub master: String,
    /// Location of artifactory slave (for fetching artifacts)
    pub slave: String,
    /// Release group name (for API queries)
    pub release: String,
    /// Virtual group (for downloads)
    pub vgroup: String,
    /// Optional publish credentials
    pub credentials: Option<Credentials>,
}


// Need these to query for stored artifacts:
// This query has tons of info, but we only care about the version
// And the version is encoded in children.uri with leading slash
#[derive(Deserialize)]
struct ArtifactoryVersion {
    uri: String, // folder: bool,
}
#[derive(Deserialize)]
struct ArtifactoryStorageResponse {
    children: Vec<ArtifactoryVersion>,
}

// simple request body fetcher
fn hyper_req(url: &str) -> LalResult<String> {
    let client = Client::new();
    let mut res = client.get(url).send()?;
    if res.status != hyper::Ok {
        return Err(CliError::BackendFailure(format!("GET request with {}", res.status)));
    }
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    Ok(body)
}

/// Query the Artifactory storage api
///
/// This will get, then parse all results as u32s, and return this list.
/// This assumes versoning is done via a single integer.
fn get_storage_versions(uri: &str) -> LalResult<Vec<u32>> {
    debug!("GET {}", uri);

    let resp = hyper_req(uri).map_err(|e| {
            warn!("Failed to GET {}: {}", uri, e);
            CliError::BackendFailure("No version information found on API".into())
        })?;

    trace!("Got body {}", resp);

    let res: ArtifactoryStorageResponse = serde_json::from_str(&resp)?;
    let builds: Vec<u32> = res.children
        .iter()
        .map(|r| r.uri.as_str())
        .map(|r| r.trim_matches('/'))
        .filter_map(|b| b.parse().ok())
        .collect();
    Ok(builds)
}

// artifactory extra headers
header! {(XCheckSumDeploy, "X-Checksum-Deploy") => [String]}
header! {(XCheckSumSha1, "X-Checksum-Sha1") => [String]}

/// Upload a tarball to artifactory
///
/// This is using a http basic auth PUT to artifactory using config credentials.
fn upload_artifact(arti: &ArtifactoryConfig, uri: &str, f: &mut File) -> LalResult<()> {
    if let Some(creds) = arti.credentials.clone() {
        let client = Client::new();

        let mut buffer: Vec<u8> = Vec::new();
        f.read_to_end(&mut buffer)?;

        let full_uri = format!("{}/{}/{}", arti.slave, arti.release, uri);
        // NB: will crash if invalid credentials or network failures atm
        // TODO: add better error handling

        let mut sha = sha1::Sha1::new();
        sha.update(&buffer);

        let auth = Authorization(Basic {
            username: creds.username,
            password: Some(creds.password),
        });

        // upload the artifact
        let resp = client.put(&full_uri[..])
            .header(auth.clone())
            .body(&buffer[..])
            .send()?;
        trace!("resp={:?}", resp);
        assert_eq!(resp.status, StatusCode::Created);

        // do another request to get the hash on artifactory
        let reqsha = client.put(&full_uri[..])
            .header(XCheckSumDeploy("true".into()))
            .header(XCheckSumSha1(sha.digest().to_string()))
            .header(auth)
            .send()?;
        trace!("resp={:?}", reqsha);
        assert_eq!(reqsha.status, StatusCode::Created);

        Ok(())
    } else {
        Err(CliError::MissingBackendCredentials)
    }
}

/// Get the maximal version number from the storage api
fn get_storage_as_u32(uri: &str) -> LalResult<u32> {
    if let Some(&latest) = get_storage_versions(uri)?.iter().max() {
        Ok(latest)
    } else {
        Err(CliError::BackendFailure("No version information found on API".into()))
    }
}

// The URL for a component tarball stored in the default artifactory location
fn get_dependency_url_default(art_cfg: &ArtifactoryConfig, name: &str, version: u32) -> String {
    let tar_url = format!("{}/{}/{}/{}/{}.tar.gz",
                          art_cfg.slave,
                          art_cfg.vgroup,
                          name,
                          version.to_string(),
                          name);

    trace!("Inferring tarball location as {}", tar_url);
    tar_url
}

// The URL for a component tarball under the one of the environment trees
fn get_dependency_env_url(art_cfg: &ArtifactoryConfig,
                          name: &str,
                          version: u32,
                          env: &str)
                          -> String {
    let tar_url = format!("{}/{}/env/{}/{}/{}/{}.tar.gz",
                          art_cfg.slave,
                          art_cfg.vgroup,
                          env,
                          name,
                          version.to_string(),
                          name);

    trace!("Inferring tarball location as {}", tar_url);
    tar_url
}

fn get_dependency_url(art_cfg: &ArtifactoryConfig,
                      name: &str,
                      version: u32,
                      env: Option<&str>)
                      -> String {
    if let Some(e) = env {
        get_dependency_env_url(art_cfg, name, version, e)
    } else {
        // This is only used by lal export without -e
        get_dependency_url_default(art_cfg, name, version)
    }
}

fn get_dependency_url_latest(art_cfg: &ArtifactoryConfig,
                             name: &str,
                             env: Option<&str>)
                             -> LalResult<Component> {
    let url = format!("{}/api/storage/{}/{}",
                      art_cfg.master,
                      art_cfg.release,
                      name);
    let v = get_storage_as_u32(&url)?;

    debug!("Found latest version as {}", v);
    Ok(Component {
        tarball: get_dependency_url(art_cfg, name, v, env),
        version: v,
        name: name.into(),
    })
}

// This queries the API for the default location
// if a default exists, then all our current multi-builds must exist
fn get_latest_versions(art_cfg: &ArtifactoryConfig,
                       name: &str,
                       env: Option<&str>)
                       -> LalResult<Vec<u32>> {
    let url = match env {
        Some(e) => {
            format!("{}/api/storage/{}/{}/{}/{}",
                    art_cfg.master,
                    art_cfg.release,
                    "env",
                    e,
                    name)
        }
        None => {
            format!("{}/api/storage/{}/{}",
                    art_cfg.master,
                    art_cfg.release,
                    name)
        }
    };
    get_storage_versions(&url)
}

/// Main entry point for install
fn get_tarball_uri(art_cfg: &ArtifactoryConfig,
                   name: &str,
                   version: Option<u32>,
                   env: Option<&str>)
                   -> LalResult<Component> {
    if let Some(v) = version {
        Ok(Component {
            tarball: get_dependency_url(art_cfg, name, v, env),
            version: v,
            name: name.into(),
        })
    } else {
        get_dependency_url_latest(art_cfg, name, env)
    }
}

/// Entry point for `lal::upgrade`
///
/// This mostly duplicates the behaviour in `get_storage_as_u32`, however,
/// it is parsing the version as a `semver::Version` struct rather than a u32.
fn find_latest_lal_version(art_cfg: &ArtifactoryConfig) -> LalResult<Version> {
    let uri = format!("{}/api/storage/{}/lal", art_cfg.master, art_cfg.release);
    debug!("GET {}", uri);
    let resp = hyper_req(&uri).map_err(|e| {
            warn!("Failed to GET {}: {}", uri, e);
            CliError::BackendFailure("No version information found on API".into())
        })?;
    trace!("Got body {}", resp);

    let res: ArtifactoryStorageResponse = serde_json::from_str(&resp)?;
    let latest: Option<Version> = res.children
        .iter()
        .map(|r| r.uri.trim_matches('/').to_string())
        .inspect(|v| trace!("Found lal version {}", v))
        .filter_map(|v| Version::parse(&v).ok())
        .max(); // Semver::Version implements an order

    if let Some(l) = latest {
        Ok(l)
    } else {
        warn!("Failed to parse version information from artifactory storage api for lal");
        Err(CliError::BackendFailure("No version information found on API".into()))
    }
}


use super::{Backend, Component};

/// Everything we need for Artifactory to implement the Backend trait
pub struct ArtifactoryBackend {
    /// Artifactory config and credentials
    pub config: ArtifactoryConfig,
    /// Cache directory
    pub cache: String,
}
impl ArtifactoryBackend {
    pub fn new(cfg: &ArtifactoryConfig, cache: &str) -> Self {
        ArtifactoryBackend {
            config: cfg.clone(),
            cache: cache.into(),
        }
    }
}

/// Artifact backend trait for `ArtifactoryBackend`
///
/// This is intended to be used by the caching trait `CachedBackend`, but for
/// specific low-level use cases, these methods can be used directly.
impl Backend for ArtifactoryBackend {
    fn get_versions(&self, name: &str, loc: Option<&str>) -> LalResult<Vec<u32>> {
        get_latest_versions(&self.config, name, loc)
    }

    fn get_latest_version(&self, name: &str, loc: Option<&str>) -> LalResult<u32> {
        let latest = get_dependency_url_latest(&self.config, name, loc)?;
        Ok(latest.version)
    }

    fn get_tarball_url(&self,
                       name: &str,
                       version: Option<u32>,
                       loc: Option<&str>)
                       -> LalResult<Component> {
        get_tarball_uri(&self.config, name, version, loc)
    }

    fn upload_file(&self, uri: &str, f: &mut File) -> LalResult<()> {
        upload_artifact(&self.config, uri, f)
    }

    fn get_latest_lal_version(&self) -> LalResult<Version> {
        find_latest_lal_version(&self.config)
    }

    fn get_lal_upgrade_url(&self) -> String {
        format!("{}/{}/lal/latest/lal.tar",
                &self.config.slave,
                &self.config.vgroup)
    }

    fn get_cache_dir(&self) -> String {
        self.cache.clone()
    }
}
