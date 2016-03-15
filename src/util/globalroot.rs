/// Globalroot shim to get components from
use std::io;
use regex::Regex;

use install::Component;
use errors::{CliError, LalResult};

fn get_latest(uri: &str) -> Option<u32> {
    use curl::http;

    debug!("GET {}", uri);
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

        debug!("Parsed version: {} from {}", version, uri);
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

fn get_blob(uri: &str) -> Option<String> {
    use curl::http;

    debug!("GET {}", uri);
    let resp = http::handle().get(uri).exec().unwrap();

    if resp.get_code() == 200 {
        let body = String::from_utf8_lossy(resp.get_body());
        // trace!("resp {}", body);

        // Assume yaml is sane for now as this is a temporary hack:
        // Since yaml is a temporary interface, this eludes the need for a yaml parser
        let re = Regex::new(r"blob: (.{64})").unwrap();
        if re.is_match(&body) {
            let blob = re.captures(&body).unwrap().at(1).unwrap().to_string();
            debug!("Found blob: {}", blob);

            // split the urls into chunks of 4
            let mut splits = vec![];
            for i in 0..16 {
                splits.push(&blob[4 * i..4 * (i + 1)]);
            }
            return Some(splits.join("/"));
        }
    }
    None
}

fn get_dependency_url_latest(name: &str, target: &str) -> LalResult<Component> {
    let globalroot = "http://builds.lal.cisco.com/globalroot/ARTIFACTS";

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
        debug!("Found latest version as {}", v);
        get_dependency_url(name, target.as_ref(), v).map(|uri| {
            Component {
                tarball: uri,
                version: v,
                name: name.to_string(),
            }
        })
    } else {
        Err(CliError::GlobalRootFailure("No tarball at corresponding blob url"))
    }
}

fn get_dependency_url(name: &str, target: &str, version: u32) -> LalResult<String> {
    let globalroot = "http://builds.lal.cisco.com/globalroot/ARTIFACTS";

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

    if let Some(blob) = get_blob(&cloud_yurl) {
        debug!("Found corresponding blob in cloud");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else if let Some(blob) = get_blob(&def_yurl) {
        debug!("Found corresponding blob in default");
        tar_url.push_str(&blob);
        Ok(tar_url)
    } else {
        Err(CliError::GlobalRootFailure("Could not find a blob"))
    }
}

/// Main entry point for install
/// 
/// If version is given, specific yaml files on cloud, then default is searched
/// If no version given, then the latest yaml files on cloud, then default is searched
///
/// Either of these will return a blob (if the version exists)
/// This blob is then turned into a tarball url that is returned along with some metadata
pub fn get_tarball_uri(name: &str, version: Option<u32>) -> LalResult<Component> {
    let target = "ncp.amd64"; // worked with globalroot with this target
    if let Some(v) = version {
        get_dependency_url(name, target, v).map(|uri| {
            Component {
                tarball: uri,
                version: v,
                name: name.to_string(),
            }
        })
    } else {
        get_dependency_url_latest(name, target)
    }
}
