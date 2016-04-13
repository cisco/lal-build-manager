/// Globalroot shim to get components from
use std::vec::Vec;
use rustc_serialize::json;
use semver::Version;

use install::Component;
use errors::{CliError, LalResult};
// Need these to query for the latest build

#[allow(non_snake_case)]
#[derive(RustcDecodable)]
struct ArtifactoryBuild {
    uri: String, // started: String,
}
#[allow(non_snake_case)]
#[derive(RustcDecodable)]
struct ArtifactoryBuildResponse {
    buildsNumbers: Vec<ArtifactoryBuild>, // uri: String,
}

fn get_latest(uri: &str) -> LalResult<u32> {
    use curl::http;

    debug!("GET {}", uri);
    let resp = try!(http::handle().get(uri).exec().map_err(|e| {
        warn!("Failed to GET {}: {}", uri, e);
        CliError::ArtifactoryFailure("GET build request failed")
    }));


    if resp.get_code() == 200 {
        let body = String::from_utf8_lossy(resp.get_body());
        trace!("Got body {}", body);

        let res: ArtifactoryBuildResponse = try!(json::decode(&body));
        let build: Option<u32> = res.buildsNumbers
                                    .iter()
                                    .map(|r| r.uri.as_str())
                                    .map(|r| r.trim_matches('/'))
                                    .filter_map(|b| b.parse().ok())
                                    .max();

        if let Some(nr) = build {
            return Ok(nr);
        }
    }
    // TODO: handle other error codes better
    Err(CliError::ArtifactoryFailure("No version information found on API"))
}

fn get_dependency_url(name: &str, version: u32) -> LalResult<String> {
    let artifactory = "http://engci-maven.cisco.com/artifactory/CME-group";
    let tar_url = [artifactory,
                   name,
                   version.to_string().as_str(),
                   format!("{}.tar.gz", name).as_str()]
        .join("/");
    debug!("Inferring tarball location as {}", tar_url);
    Ok(tar_url)
}

fn get_dependency_url_latest(name: &str) -> LalResult<Component> {
    let artifactory = "http://engci-maven.cisco.com/artifactory/api/build/team_CME%20::%20";
    let url = [artifactory, name].concat();

    let v = try!(get_latest(&url));

    debug!("Found latest version as {}", v);
    let c = try!(get_dependency_url(name, v).map(|uri| {
        Component {
            tarball: uri,
            version: v,
            name: name.to_string(),
        }
    }));
    Ok(c)
}


/// Main entry point for install
pub fn get_tarball_uri(name: &str, version: Option<u32>) -> LalResult<Component> {
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

pub fn find_latest_lal_version() -> LalResult<Version> {
    use curl::http;
    let uri = "http://engci-maven.cisco.com/artifactory/api/storage/CME-group/lal";

    debug!("GET {}", uri);
    let resp = try!(http::handle().get(uri).exec().map_err(|e| {
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
