pub fn install(xs: Vec<&str>, save: bool) {
    println!("installing specific deps {:?} {}", xs, save);
}

fn download_to_path(uri: &str, save: &str) {
    use std::fs::File;
    use std::error::Error;
    use std::path::Path;
    use curl::http;
    use std::io::prelude::*;

    println!("GET {}", uri);
    let resp = http::handle().get(uri).exec().unwrap();

    if resp.get_code() == 200 {
        let r = resp.get_body();

        let path = Path::new(save);
        let display = path.display();

        // TODO: reduce error handling here, overkill
        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, Error::description(&why)),
            Ok(file) => file,
        };

        match file.write_all(r) {
            Err(why) => {
                panic!("couldn't write to {}: {}",
                       display,
                       Error::description(&why))
            }
            Ok(_) => println!("-> {}", display),
        }
    }
}

pub fn install_all() {
    println!("plain install");
    download_to_path("http://i.imgur.com/jAAwK7o.jpg", "barn.jpg")
}
