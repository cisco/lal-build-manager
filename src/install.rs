use std::io;

pub fn install(xs: Vec<&str>, save: bool) {
    println!("installing specific deps {:?} {}", xs, save);
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

pub fn install_all() {
    println!("plain install");
    let _ = download_to_path("http://i.imgur.com/jAAwK7o.jpg", "barn.jpg");
}
