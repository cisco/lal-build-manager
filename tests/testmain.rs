extern crate lal;

#[macro_use]
extern crate log;
extern crate loggerv;
extern crate walkdir;

use std::env;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::process::Command;
use std::io::prelude::*;
use walkdir::WalkDir;

//use loggerv::init_with_verbosity;
use lal::{Config, Manifest};

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
    let has_docker = true;
    let num_tests = if has_docker { 15 } else { 11 };

    println!("# lal tests");
    println!("1..{}", num_tests);
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
    has_config_and_manifest();
    println!("ok {} has_config_and_manifest", i);
    // assume we have manifest and config after this point

    i += 1;
    update_save();
    println!("ok {} update_save", i);

    i += 1;
    verify_checks();
    println!("ok {} verify_checks", i);

    if has_docker {
        i += 1;
        shell_echo();
        println!("ok {} shell_echo", i);

        i += 1;
        shell_permissions();
        println!("ok {} shell_permissions", i);

        i += 1;
        build_stash_and_update_from_stash();
        println!("ok {} build_stash_and_update_from_stash", i);

        i += 1;
        run_scripts();
        println!("ok {} run_scripts", i);

        i += 1;
        status_on_experimentals();
        println!("ok {} status_on_experimentals", i);
    }

    i += 1;
    upgrade_does_not_fail();
    println!("ok {} upgrade_does_not_fail", i);

    i += 1;
    export_check();
    println!("ok {} export_check", i);

    i += 1;
    clean_check();
    println!("ok {} clean_check", i);
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

    let r = lal::configure(false, true, Some("edonusdevelopers/muslrust:1.8.0-2016-04-15"));
    assert!(r.is_ok(), "configure succeeded");

    let cfg = Config::read();
    assert!(cfg.is_ok(), "config exists now");
}

// Create manifest
fn init_force() {
    let m1 = Manifest::read();
    assert!(m1.is_err(), "no manifest at this point");

    let m2 = lal::init(false);
    assert!(m2.is_ok(), "could init without force param");

    let m3 = lal::init(true);
    assert!(m3.is_ok(), "could re-init with force param");

    let m4 = lal::init(false);
    assert!(m4.is_err(), "could not re-init without force ");
}

// Tests need to be run in a directory with a manifest
// and ~/.lal + lalrc must exist
fn has_config_and_manifest() {
    let ldir = lal_dir();
    assert!(ldir.is_dir(), "have laldir");

    let cfg = Config::read();
    chk::is_ok(cfg, "could read config");

    let manifest = Manifest::read();
    chk::is_ok(Manifest::read(), "could read manifest");

    // There is no INPUT yet, but we have no dependencies, so this should work:
    let r = lal::verify(&manifest.unwrap());
    chk::is_ok(r, "could verify after install");
}

// add some dependencies
fn update_save() {
    let mf1 = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();

    // gtest savedev
    let ri = lal::update(mf1, &cfg, vec!["gtest".to_string()], false, true);
    chk::is_ok(ri, "could update gtest and save as dev");

    // three main deps (and re-read manifest to avoid overwriting devedps)
    let mf2 = Manifest::read().unwrap();
    let updates = vec!["libyaml".to_string(), "yajl".to_string(), "libwebsockets".to_string()];
    let ri = lal::update(mf2, &cfg, updates, true, false);
    chk::is_ok(ri, "could update libyaml and save");

    // verify update-all --save
    let mf3 = Manifest::read().unwrap();
    let ri = lal::update_all(mf3, &cfg, true, false);
    chk::is_ok(ri, "could update all and --save");

    // verify update-all --save --dev
    let mf4 = Manifest::read().unwrap();
    let ri = lal::update_all(mf4, &cfg, false, true);
    chk::is_ok(ri, "could update all and --save --dev");
}

fn verify_checks() {
    let cfg = Config::read().unwrap();
    let mf = Manifest::read().unwrap();

    let r = lal::verify(&mf);
    assert!(r.is_ok(), "could verify after install");

    let gtest = Path::new(&env::current_dir().unwrap()).join("INPUT").join("gtest");
    // clean folders and verify it fails
    let yajl = Path::new(&env::current_dir().unwrap()).join("INPUT").join("yajl");
    fs::remove_dir_all(&yajl).unwrap();

    let r2 = lal::verify(&mf);
    assert!(r2.is_err(), "verify failed after fiddling");

    // re-install everything
    let rall = lal::fetch(&mf, cfg, true);
    assert!(rall.is_ok(), "install all succeeded");
    assert!(yajl.is_dir(), "yajl was reinstalled from manifest");
    assert!(!gtest.is_dir(), "gtest was not reinstalled from manifest with core");


    let r3 = lal::verify(&mf);
    assert!(r3.is_ok(), "verify ok again");
}

// Shell tests
fn shell_echo() {
    let cfg = Config::read().unwrap();
    let r = lal::docker_run(&cfg, vec!["echo".to_string(), "# echo from docker".to_string()], false, false, false);
    assert!(r.is_ok(), "shell echoed");
}
fn shell_permissions() {
    let cfg = Config::read().unwrap();
    let r = lal::docker_run(&cfg, vec!["touch".to_string(), "README.md".to_string()], false, false, false);
    assert!(r.is_ok(), "could touch files in container");
}

fn build_stash_and_update_from_stash() {
    let mf = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();

    {
        let mut f = File::create("./BUILD").unwrap();
        write!(f, "#!/bin/bash\necho hi > OUTPUT/test.txt\n").unwrap();
        Command::new("chmod").arg("+x").arg("BUILD").output().unwrap();
    } // scope ensures file is not busy before lal::build

    let r = lal::build(&cfg, &mf, None, None, true, None, true, false);
    assert!(r.is_ok(), "could run lal build and could make tarball");

    // lal stash testmain
    let r2 = lal::stash(&cfg, &mf, "testmain");
    assert!(r2.is_ok(), "could stash lal build artifact");

    // lal update lal=testmain
    let ri = lal::update(mf.clone(), &cfg, vec!["lal=testmain".to_string()], false, false);
    chk::is_ok(ri, "could update lal from stash");
}

fn run_scripts() {
    {
        Command::new("mkdir").arg("-p").arg(".lal/scripts").output().unwrap();
        let mut f = File::create("./.lal/scripts/subroutine").unwrap();
        write!(f, "#!/bin/bash\necho hi $1 $2\n").unwrap();
        Command::new("chmod").arg("+x").arg(".lal/scripts/subroutine").output().unwrap();
    }
    let cfg = Config::read().unwrap();
    let r = lal::script(&cfg, "subroutine", vec!["hi", "there"], false);
    assert!(r.is_ok(), "could run subroutine script");
}

fn status_on_experimentals() {
    let mf = Manifest::read().unwrap();
    // both of these should return errors, but work
    let r = lal::status(&mf, false);
    assert!(r.is_err(), "status should complain at experimental deps");
    let r = lal::status(&mf, true);
    assert!(r.is_err(), "status should complain at experimental deps");
}

fn upgrade_does_not_fail() {
    let cfg = Config::read().unwrap();
    let uc = lal::upgrade_check(&cfg, true);
    assert!(uc.is_ok(), "could perform upgrade check");
    let upgraded = uc.unwrap();
    assert!(!upgraded, "we never have upgrades in the tip source tree");
}

fn clean_check() {
    let cfg = Config::read().unwrap();
    let r = lal::clean(&cfg, 1);
    assert!(r.is_ok(), "could run partial lal cleanup");

    // scan cache dir
    let mut dirs = WalkDir::new(&cfg.cache)
        .min_depth(3)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir());

    let first = dirs.next();
    assert!(first.is_some(), "some artifacts cached since last time");

    // run check again cleaning everything
    let r = lal::clean(&cfg, 0);
    assert!(r.is_ok(), "could run full lal cleanup");

    // scan cache dir
    let mut dirs2 = WalkDir::new(&cfg.cache)
        .min_depth(3)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir());

    let first2 = dirs2.next();
    assert!(first2.is_none(), "no artifacts left in cache");
}

fn export_check() {
    let cfg = Config::read().unwrap();

    let r = lal::export(&cfg, "gtest=6", Some("tests"));
    assert!(r.is_ok(), "could export gtest=6 into subdir");

    let r2 = lal::export(&cfg, "libcurl", None);
    assert!(r2.is_ok(), "could export latest libcurl into PWD");

    let gtest = Path::new(&env::current_dir().unwrap()).join("tests").join("gtest.tar.gz");
    assert!(gtest.is_file(), "gtest was copied correctly");
    let libcurl = Path::new(&env::current_dir().unwrap()).join("libcurl.tar.gz");
    assert!(libcurl.is_file(), "libcurl was copied correctly");
}
