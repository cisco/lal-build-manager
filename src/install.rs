use std::io;

// temporary fn
fn get_dependency_url(name: &str) -> String {
    use std::collections::HashMap;
    // Shim globalroot
    let blobstore = "http://builds.lal.cisco.com/globalroot/ARTIFACTS/.blobs/";
    let mut xs = HashMap::new();
    xs.insert("ciscossl",
              "878a/2c0a/59d5/6fb1/fed0/6be7/52ac/2fb9/d09b/6787/78d6/eecb/0497/fea9/56ed/6e44");
    xs.insert("expat",
              "94f1/dde9/4bc1/f534/2488/4675/fe50/f49f/a7fc/f709/65e7/eae5/d533/aa58/9fa3/c698");
    xs.insert("freetype",
              "36cf/ff9c/9f71/7fae/e0d2/4f3a/48bf/1e9b/d26d/1d57/9458/4df1/839a/18e2/050b/2e02");
    xs.insert("fribidi",
              "167f/2220/9d69/19e7/be8c/6efe/6ab5/f997/3cc7/77bf/0be8/825e/295c/69ac/0510/3756");
    xs.insert("gtest",
              "a687/3332/45f1/d1dc/df01/4b47/1625/9812/a66e/c6e4/f362/3625/f7fd/c807/59d7/ac54");
    xs.insert("libarchive",
              "253d/ac1a/786a/01dd/202e/f373/99db/c284/16c4/3c8c/778c/3203/c5b5/b39a/88ab/67dd");
    xs.insert("libcurl",
              "b2ff/227b/c96b/2c97/e1c3/9ea0/f4a5/f556/b47e/557f/b86c/e471/4ed3/678b/a825/6366");
    xs.insert("libldns",
              "1afd/4334/1a6a/0912/983c/f5ab/403c/2075/f5b3/b22b/55a0/b8fe/7987/be1a/7db2/7708");
    xs.insert("liblzma",
              "d6d2/51dc/b95b/7525/6068/9db2/108a/0d2b/854b/ff98/74b3/b2af/390c/420b/4351/0c32");
    xs.insert("libpng",
              "fb77/bff4/5527/adb6/fd73/c22c/ee43/043e/78b9/d748/6f78/02c3/6d97/864a/0398/f57c");
    xs.insert("libunbound",
              "73d5/2fed/e626/382c/ceed/bfe4/98d5/c483/7f46/48bb/494f/11fb/b09b/2145/dee4/5fbc");
    xs.insert("libwebsockets",
              "aebc/cbb6/e4d9/b602/c0a6/9303/79f6/a90b/7a8b/f767/ce8d/b95a/3ca8/b505/cb7b/4603");
    xs.insert("libyaml",
              "ef2b/38ad/317a/f403/320f/4cf2/6ce0/a085/2069/518d/d15c/6728/8f17/65af/4f4a/fbbb");
    xs.insert("p7zip",
              "68c8/761f/9d47/4fdb/544c/2891/5ab6/f562/d6f4/9ae2/b071/c39d/3a38/9703/bdb0/5e4f");
    xs.insert("yajl",
              "33e3/bdc7/02e4/70e3/49a5/dbbb/a325/54f5/9948/9c6e/c530/424c/f664/07e8/ea61/e263");
    xs.insert("zlib",
              "44bf/2094/e83a/9767/89c1/f331/e645/ada1/4efe/e804/6ca7/ce34/0d75/6727/bab6/148a");
    if xs.contains_key(&name) {
        let mut url = blobstore.to_string();
        url.push_str(&xs[&name]);
        return url;
    }
    return "http://lolcathost".to_string();
}

pub fn install(xs: Vec<&str>, save: bool, savedev: bool) {
    println!("installing specific deps {:?} {} {}", xs, save, savedev);
    for v in &xs {
        println!("fetching dependency {}", v);
        let uri = get_dependency_url(&v);
        println!("found url {}", uri);
        let mut name = v.to_string();
        name.push_str(".tar.gz");
        let _ = download_to_path(&uri, &name);

    }
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

    println!("plain install");
    let manifest = init::read_manifest().unwrap();
    println!("dependencies: {:?}", manifest.dependencies);
    for (k, v) in &manifest.dependencies {
        println!("installing {} {}", k, v);
        let uri = get_dependency_url(&k);
        println!("uri {}", uri);
    }

    if dev {
        println!("devDependencies: {:?}", manifest.devDependencies);
    }
}
