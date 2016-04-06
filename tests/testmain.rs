extern crate lal;

#[macro_use]
extern crate log;
extern crate loggerv;

use std::env;
use std::path::{Path, PathBuf};
use std::fs;

//use loggerv::init_with_verbosity;
use lal::{configure, install, verify, init, shell, build, Config, Manifest, LalResult};

// TODO: macroify this stuff

mod chk {
    use std::fmt::Display;
    use std::process;
    // TODO: don't need to move T into here, but since they are joined..
    pub fn is_ok<T, E: Display>(x: Result<T, E>, name: &str) {
        let _ = x.map_err(|e| {
            println!("Bail out! {} failed with '{}'", name, e);
            process::exit(1);
        });
    }
}
//fn assert_err<T>(x: LalResult<T>, name: &str) {
//    let _ = x.map(|v| {
//        println!("Bail out! {} unexpected ok: {}", name, v);
//        process::exit(1);
//    });
//}

fn main() {
    //init_with_verbosity(0).unwrap();
    println!("# lal tests");
    println!("1..11");
    let mut i = 0;

    i += 1;
    kill_laldir();
    println!("ok {} kill_laldir", i);

    i += 1;
    kill_input();
    println!("ok {} kill_input", i);

    i += 1;
    kill_manifest();
    println!("ok {} kill_manifest", i);

    i += 1;
    configure_yes();
    println!("ok {} configure_yes", i);

    i += 1;
    init_force();
    println!("ok {} init_force", i);

    i += 1;
    sanity();
    println!("ok {} sanity", i);

    // assume we have manifest and config after this point

    i += 1;
    install_save();
    println!("ok {} install_save", i);

    i += 1;
    verify_checks();
    println!("ok {} verify_checks", i);

    i += 1;
    shell_echo();
    println!("ok {} shell_echo", i);

    i += 1;
    shell_permissions();
    println!("ok {} shell_permissions", i);

    i += 1;
    build_tar();
    println!("ok {} build_tar", i);
}

fn lal_dir() -> PathBuf {
    let home = env::home_dir().unwrap();
    Path::new(&home).join(".lal/")
}

// Start from scratch
fn kill_laldir() {
    let ldir = lal_dir();
    if ldir.is_dir() {
        fs::remove_dir_all(&ldir).unwrap();
    }
    assert_eq!(ldir.is_dir(), false);
}
fn kill_input() {
    let input = Path::new(&env::current_dir().unwrap()).join("INPUT");
    if input.is_dir() {
        fs::remove_dir_all(&input).unwrap();
    }
    assert_eq!(input.is_dir(), false);
}
fn kill_manifest() {
    let manifest = Path::new(&env::current_dir().unwrap()).join("manifest.json");
    if manifest.is_file() {
        fs::remove_file(&manifest).unwrap();
    }
    assert_eq!(manifest.is_file(), false);
}

// Create lalrc
fn configure_yes() {
    let config = Config::read();
    assert!(config.is_err(), "no lalrc at this point");

    let r = configure::configure(false, true);
    assert!(r.is_ok(), "configure succeeded");

    let cfg = Config::read();
    assert!(cfg.is_ok(), "config exists now");
}

// Create manifest
fn init_force() {
    let m1 = Manifest::read();
    assert!(m1.is_err(), "no manifest at this point");

    let m2 = init::init(false);
    assert!(m2.is_ok(), "could init without force param");

    let m3 = init::init(true);
    assert!(m3.is_ok(), "could re-init with force param");

    let m4 = init::init(false);
    assert!(m4.is_err(), "could not re-init without force ");
}

// Tests need to be run in a directory with a manifest
// and ~/.lal + lalrc must exist
fn sanity() {
    let ldir = lal_dir();
    assert!(ldir.is_dir(), "have laldir");

    let cfg = Config::read();
    chk::is_ok(cfg, "could read config");

    let manifest = Manifest::read();
    chk::is_ok(Manifest::read(), "could read manifest");

    // There is no INPUT yet, but we have no dependencies, so this should work:
    let r = verify::verify(manifest.unwrap());
    chk::is_ok(r, "could verify after install");
}

// add some dependencies
fn install_save() {
    let mf1 = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();

    // gtest savedev
    let ri = install::install(mf1, cfg.clone(), vec!["gtest"], false, true);
    chk::is_ok(ri, "could install gtest and save as dev");

    // three main deps (and re-read manifest to avoid overwriting devedps)
    let mf2 = Manifest::read().unwrap();
    let ri = install::install(mf2, cfg.clone(), vec!["libyaml", "yajl", "libwebsockets"], true, false);
    chk::is_ok(ri, "could install libyaml and save");
}

//fn component_dir(name: &str) -> PathBuf {
//    Path::new(&env::current_dir().unwrap()).join("INPUT").join(&name).join("ncp.amd64")
//}

fn verify_checks() {
    let cfg = Config::read().unwrap();

    let r = verify::verify(Manifest::read().unwrap());
    assert!(r.is_ok(), "could verify after install");

    // clean folders and verify it fails
    let yajl = Path::new(&env::current_dir().unwrap()).join("INPUT").join("yajl");
    fs::remove_dir_all(&yajl).unwrap();

    let r2 = verify::verify(Manifest::read().unwrap());
    assert!(r2.is_err(), "verify failed after fiddling");

    // re-install everything
    let rall = install::install_all(Manifest::read().unwrap(), cfg, true);
    assert!(rall.is_ok(), "install all succeeded");
    assert!(yajl.is_dir(), "yajl was reinstalled from manifest");

    let r3 = verify::verify(Manifest::read().unwrap());
    assert!(r3.is_ok(), "verify ok again");
}

// Shell tests
fn shell_echo() {
    let cfg = Config::read().unwrap();
    let r = shell::docker_run(&cfg, vec!["echo".to_string(), "# echo from docker".to_string()], false);
    assert!(r.is_ok(), "shell echoed");
}
fn shell_permissions() {
    let cfg = Config::read().unwrap();
    let r = shell::docker_run(&cfg, vec!["touch".to_string(), "README.md".to_string()], false);
    assert!(r.is_ok(), "could touch files in container");
}

fn build_tar() {
    let mf = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();

    // TODO: need to have a BUILD script that actually creates a tarball in OUTPUT
    // currently tests work because I have such a BUILD, but don't want to commit it
    let r = build::build(&cfg, &mf, None, None, true, None);
    assert!(r.is_ok(), "could run lal build and could make tarball");
}
