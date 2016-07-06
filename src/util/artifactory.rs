/// Globalroot shim to get components from
use std::vec::Vec;
use rustc_serialize::json;
use semver::Version;

use install::Component;
use configure::ArtifactoryConfig;
use errors::{CliError, LalResult};

// Need these to query for stored artifacts:
// This query has tons of info, but we only care about the version
// And the version is encoded in children.uri with leading slash
#[derive(RustcDecodable)]
struct ArtifactoryVersion {
    uri: String, // folder: bool,
}
#[derive(RustcDecodable)]
struct ArtifactoryStorageResponse {
    children: Vec<ArtifactoryVersion>,
}

/// Query the Artifactory storage api
///
/// This will get, then parse all results as u32s, and return this list.
/// This assumes versoning is done via a single integer.
fn get_storage_versions(uri: &str) -> LalResult<Vec<u32>> {
    use curl::http;

    debug!("GET {}", uri);
    let resp = try!(http::handle().get(uri).exec().map_err(|e| {
        warn!("Failed to GET {}: {}", uri, e);
        CliError::ArtifactoryFailure("GET build request failed")
    }));


    if resp.get_code() == 200 {
        let body = String::from_utf8_lossy(resp.get_body());
        trace!("Got body {}", body);

        let res: ArtifactoryStorageResponse = try!(json::decode(&body));
        let builds: Vec<u32> = res.children
                                  .iter()
                                  .map(|r| r.uri.as_str())
                                  .map(|r| r.trim_matches('/'))
                                  .filter_map(|b| b.parse().ok())
                                  .collect();

        return Ok(builds);
    }
    // TODO: handle other error codes better
    Err(CliError::ArtifactoryFailure("No version information found on API"))
}

/// Get the maximal version number from the storage api
fn get_storage_as_u32(uri: &str) -> LalResult<u32> {
    if let Some(&latest) = try!(get_storage_versions(uri)).iter().max() {
        Ok(latest)
    } else {
        Err(CliError::ArtifactoryFailure("No version information found on API"))
    }
}

fn get_dependency_url(art_cfg: &ArtifactoryConfig, name: &str, version: u32) -> String {
    let tar_url = format!("{}/{}/{}/{}/{}.tar.gz",
                          art_cfg.server,
                          art_cfg.group,
                          name,
                          version.to_string(),
                          name);

    trace!("Inferring tarball location as {}", tar_url);
    tar_url
}

fn get_dependency_url_latest(art_cfg: &ArtifactoryConfig, name: &str) -> LalResult<Component> {
    let url = format!("{}/api/storage/{}/{}", art_cfg.server, art_cfg.group, name);
    let v = try!(get_storage_as_u32(&url));

    debug!("Found latest version as {}", v);
    Ok(Component {
        tarball: get_dependency_url(art_cfg, name, v),
        version: v,
        name: name.to_string(),
    })
}

pub fn get_latest_versions(art_cfg: &ArtifactoryConfig, name: &str) -> LalResult<Vec<u32>> {
    let url = format!("{}/api/storage/{}/{}", art_cfg.server, art_cfg.group, name);
    get_storage_versions(&url)
}



/// Main entry point for install
pub fn get_tarball_uri(art_cfg: &ArtifactoryConfig,
                       name: &str,
                       version: Option<u32>)
                       -> LalResult<Component> {
    if let Some(v) = version {
        Ok(Component {
            tarball: get_dependency_url(art_cfg, name, v),
            version: v,
            name: name.to_string(),
        })
    } else {
        get_dependency_url_latest(art_cfg, name)
    }
}

/// Entry point for lal::upgrade
///
/// This mostly duplicates the behaviour in `get_storage_as_u32`, however,
/// it is parsing the version as a semver::Version struct rather than a u32.
pub fn find_latest_lal_version(art_cfg: &ArtifactoryConfig) -> LalResult<Version> {
    use curl::http;
    let uri = format!("{}/api/storage/{}/lal", art_cfg.server, art_cfg.group);

    debug!("GET {}", uri);
    let resp = try!(http::handle().get(uri.as_str()).exec().map_err(|e| {
        warn!("Failed to GET {}: {}", uri, e);
        CliError::ArtifactoryFailure("Storage request failed")
    }));


    if resp.get_code() == 200 {
        let body = String::from_utf8_lossy(resp.get_body());
        trace!("Got body {}", body);

        let res: ArtifactoryStorageResponse = try!(json::decode(&body));
        let latest: Option<Version> = res.children
                                         .iter()
                                         .map(|r| r.uri.trim_matches('/').to_string())
                                         .inspect(|v| trace!("Found lal version {}", v))
                                         .filter_map(|v| Version::parse(&v).ok())
                                         .max(); // Semver::Version implements an order

        if latest.is_some() {
            return Ok(latest.unwrap());
        } else {
            warn!("Failed to parse version information from artifactory storage api for lal");
        }
    }
    Err(CliError::ArtifactoryFailure("No version information found on API"))
}
