extern crate lal;

extern crate log;
extern crate loggerv;
extern crate walkdir;

use std::env;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::process::Command;
use std::io::prelude::*;
use walkdir::WalkDir;

// use loggerv::init_with_verbosity;
use lal::*;

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
// fn assert_err<T>(x: LalResult<T>, name: &str) {
//    let _ = x.map(|v| {
//        println!("Bail out! {} unexpected ok: {}", name, v);
//        process::exit(1);
//    });
//

fn init_ssl() {
    use std::env;
    env::set_var("SSL_CERT_FILE", "/etc/ssl/certs/ca-certificates.crt");
}

fn main() {
    init_ssl();

    // init_with_verbosity(0).unwrap();
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
    let backend = configure_yes();
    println!("ok {} configure_yes", i);

    i += 1;
    init_force();
    println!("ok {} init_force", i);

    i += 1;
    has_config_and_manifest();
    println!("ok {} has_config_and_manifest", i);
    // assume we have manifest and config after this point

    i += 1;
    update_save(&backend);
    println!("ok {} update_save", i);

    i += 1;
    verify_checks(&backend);
    println!("ok {} verify_checks", i);

    if has_docker {
        i += 1;
        shell_echo();
        println!("ok {} shell_echo", i);

        i += 1;
        shell_permissions();
        println!("ok {} shell_permissions", i);

        i += 1;
        build_stash_and_update_from_stash(&backend);
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
    export_check(&backend);
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
    let pwd = env::current_dir().unwrap();
    let manifest = Path::new(&pwd).join("manifest.json");
    let lalsubdir = Path::new(&pwd).join(".lal");
    if manifest.is_file() {
        fs::remove_file(&manifest).unwrap();
    }
    if lalsubdir.exists() {
        fs::remove_dir_all(&lalsubdir).unwrap();
    }
    assert_eq!(manifest.is_file(), false);
}

// Create config
fn configure_yes() -> ArtifactoryBackend {
    let config = Config::read();
    assert!(config.is_err(), "no config at this point");

    let r = lal::configure(true, false, "configs/edonus.json");
    assert!(r.is_ok(), "configure succeeded");

    let cfg = Config::read();
    assert!(cfg.is_ok(), "config exists now");

    let cfgu = cfg.unwrap();

    match &cfgu.backend {
        &BackendConfiguration::Artifactory(ref art_cfg) => {
            ArtifactoryBackend::new(art_cfg, &cfgu.cache)
        }
    }
}

// Create manifest
fn init_force() {
    let cfg = Config::read().unwrap();

    let m1 = Manifest::read();
    assert!(m1.is_err(), "no manifest at this point");

    let m2 = lal::init(&cfg, false, "rust");
    assert!(m2.is_ok(), "could init without force param");

    let m3 = lal::init(&cfg, true, "rust");
    assert!(m3.is_ok(), "could re-init with force param");

    let m4 = lal::init(&cfg, false, "rust");
    assert!(m4.is_err(), "could not re-init without force ");

    let m5 = lal::init(&cfg, true, "blah");
    assert!(m5.is_err(), "could not init without valid environment");
}

// Tests need to be run in a directory with a manifest
// and ~/.lal + config must exist
fn has_config_and_manifest() {
    let ldir = lal_dir();
    assert!(ldir.is_dir(), "have laldir");

    let cfg = Config::read();
    chk::is_ok(cfg, "could read config");

    let manifest = Manifest::read();
    chk::is_ok(Manifest::read(), "could read manifest");

    // There is no INPUT yet, but we have no dependencies, so this should work:
    let r = lal::verify(&manifest.unwrap(), "centos".into(), false);
    chk::is_ok(r, "could verify after install");
}

// add some dependencies
fn update_save<T: CachedBackend + Backend>(backend: &T) {
    let mf1 = Manifest::read().unwrap();

    // gtest savedev
    let ri = lal::update(&mf1,
                         backend,
                         vec!["gtest".to_string()],
                         false,
                         true,
                         "xenial");
    chk::is_ok(ri, "could update gtest and save as dev");

    // three main deps (and re-read manifest to avoid overwriting devedps)
    let mf2 = Manifest::read().unwrap();
    let updates = vec!["libyaml".to_string(), "yajl".to_string(), "libwebsockets".to_string()];
    let ri = lal::update(&mf2, backend, updates, true, false, "xenial");
    chk::is_ok(ri, "could update libyaml and save");

    // verify update-all --save
    let mf3 = Manifest::read().unwrap();
    let ri = lal::update_all(&mf3, backend, true, false, "xenial");
    chk::is_ok(ri, "could update all and --save");

    // verify update-all --save --dev
    let mf4 = Manifest::read().unwrap();
    let ri = lal::update_all(&mf4, backend, false, true, "xenial");
    chk::is_ok(ri, "could update all and --save --dev");
}

fn verify_checks<T: CachedBackend + Backend>(backend: &T) {
    let mf = Manifest::read().unwrap();

    let r = lal::verify(&mf, "xenial".into(), false);
    assert!(r.is_ok(), "could verify after install");

    let renv1 = lal::verify(&mf, "centos".into(), false);
    assert!(renv1.is_err(), "could not verify with wrong env");
    let renv2 = lal::verify(&mf, "centos".into(), true);
    assert!(renv2.is_err(),
            "could not verify with wrong env - even with simple");

    let gtest = Path::new(&env::current_dir().unwrap()).join("INPUT").join("gtest");
    // clean folders and verify it fails
    let yajl = Path::new(&env::current_dir().unwrap()).join("INPUT").join("yajl");
    fs::remove_dir_all(&yajl).unwrap();

    let r2 = lal::verify(&mf, "xenial".into(), false);
    assert!(r2.is_err(), "verify failed after fiddling");

    // fetch --core, resyncs with core deps (removes devDeps and other extraneous)
    let rcore = lal::fetch(&mf, backend, true, "xenial");
    assert!(rcore.is_ok(), "install core succeeded");
    assert!(yajl.is_dir(), "yajl was reinstalled from manifest");
    assert!(!gtest.is_dir(),
            "gtest was was extraneous with --core => removed");

    // fetch --core also doesn't install else again
    let rcore2 = lal::fetch(&mf, backend, true, "xenial");
    assert!(rcore2.is_ok(), "install core succeeded 2");
    assert!(yajl.is_dir(), "yajl still there");
    assert!(!gtest.is_dir(), "gtest was not reinstalled with --core");

    // and it is finally installed if we ask for non-core as well
    let rall = lal::fetch(&mf, backend, false, "xenial");
    assert!(rall.is_ok(), "install all succeeded");
    assert!(gtest.is_dir(), "gtest is otherwise installed again");

    let r3 = lal::verify(&mf, "xenial", false);
    assert!(r3.is_ok(), "verify ok again");
}

// Shell tests
fn shell_echo() {
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("rust".into()).unwrap();
    let modes = ShellModes::default();
    let r = lal::docker_run(&cfg,
                            &container,
                            vec!["echo".to_string(), "# echo from docker".to_string()],
                            &DockerRunFlags::default(),
                            &modes);
    assert!(r.is_ok(), "shell echoed");
}
fn shell_permissions() {
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("rust".into()).unwrap();
    let modes = ShellModes::default();
    let r = lal::docker_run(&cfg,
                            &container,
                            vec!["touch".to_string(), "README.md".to_string()],
                            &DockerRunFlags::default(),
                            &modes);
    assert!(r.is_ok(), "could touch files in container");
}

fn build_stash_and_update_from_stash<T: CachedBackend + Backend>(backend: &T) {
    let mf = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("rust".into()).unwrap();

    {
        let mut f = File::create("./BUILD").unwrap();
        // Rust check in there to verify we can build in a rust container
        write!(f,
               "#!/bin/bash\nset -e\nwhich rustc\necho hi > OUTPUT/test.txt\n")
            .unwrap();
        Command::new("chmod").arg("+x").arg("BUILD").output().unwrap();
    } // scope ensures file is not busy before lal::build


    // we'll try with various build options further down with various deps
    let mut bopts = BuildOptions {
        name: Some("lal".into()),
        configuration: Some("release".into()),
        container: container,
        release: true,
        version: None,
        sha: None,
        force: false,
        simple_verify: false,
    };
    let modes = ShellModes::default();
    // basic build works - all deps are global at right env
    let r = lal::build(&cfg, &mf, &bopts, "xenial".into(), &modes);
    assert!(r.is_ok(), "could perform a xenial build");

    // lal stash testmain
    let rs = lal::stash(backend, &mf, "testmain");
    assert!(rs.is_ok(), "could stash lal build artifact");

    // lal update lal=testmain
    let ru = lal::update(&mf,
                         backend,
                         vec!["lal=testmain".to_string()],
                         false,
                         false,
                         "garbage"); // env not relevant for stash
    chk::is_ok(ru, "could update lal from stash");

    // basic build won't work now without simple verify
    let r1 = lal::build(&cfg, &mf, &bopts, "xenial".into(), &modes);
    assert!(r1.is_err(), "could not verify a new xenial build");
    if let Err(CliError::NonGlobalDependencies(nonglob)) = r1 {
        assert_eq!(nonglob, "lal");
    } else {
        println!("actual r1 was {:?}", r1);
        assert!(false);
    }

    bopts.simple_verify = true;
    let r2 = lal::build(&cfg, &mf, &bopts, "xenial".into(), &modes);
    assert!(r2.is_ok(), "can build with stashed deps with simple verify");


    // force will also work - even with stashed deps from wrong env
    let renv = lal::build(&cfg, &mf, &bopts, "rust".into(), &modes);
    assert!(renv.is_err(),
            "cannot build with simple verify when wrong env");
    if let Err(CliError::EnvironmentMismatch(_, compenv)) = renv {
        assert_eq!(compenv, "xenial"); // expected complaints about xenial env
    } else {
        println!("actual renv was {:?}", renv);
        assert!(false);
    }

    // settings that reflect lal build -f
    bopts.simple_verify = false;
    bopts.force = true;
    let renv2 = lal::build(&cfg, &mf, &bopts, "rust".into(), &modes);
    assert!(renv2.is_ok(), "could force build in different env");

    // additionally do a build with printonly
    let all_modes = ShellModes {
        printonly: true,
        x11_forwarding: true,
        host_networking: true,
        env_vars: vec![],
    };
    let printbuild = lal::build(&cfg, &mf, &bopts, "rust".into(), &all_modes);
    assert!(printbuild.is_ok(), "saw docker run print with X11 mounts");
}

fn run_scripts() {
    {
        Command::new("mkdir").arg("-p").arg(".lal/scripts").output().unwrap();
        let mut f = File::create("./.lal/scripts/subroutine").unwrap();
        write!(f, "main() {{ echo hi $1 $2 ;}}\n").unwrap();
        Command::new("chmod").arg("+x").arg(".lal/scripts/subroutine").output().unwrap();
    }
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("rust".into()).unwrap();
    let modes = ShellModes::default();
    let r = lal::script(&cfg, &container, "subroutine", vec!["there", "mr"], &modes, false);
    assert!(r.is_ok(), "could run subroutine script");
}

fn status_on_experimentals() {
    let mf = Manifest::read().unwrap();
    // both of these should return errors, but work
    let r = lal::status(&mf, false, false, false);
    assert!(r.is_err(), "status should complain at experimental deps");
    let r = lal::status(&mf, true, true, true);
    assert!(r.is_err(), "status should complain at experimental deps");
}

fn upgrade_does_not_fail() {
    let uc = lal::upgrade(true);
    assert!(uc.is_ok(), "could perform upgrade check");
    let upgraded = uc.unwrap();
    assert!(!upgraded, "we never have upgrades in the tip source tree");
}

fn clean_check() {
    let cfg = Config::read().unwrap();
    let r = lal::clean(&cfg.cache, 1);
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
    let r = lal::clean(&cfg.cache, 0);
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

fn export_check<T: CachedBackend + Backend>(backend: &T) {
    let r = lal::export(backend, "gtest=6", Some("tests"), None);
    assert!(r.is_ok(), "could export global gtest=6 into subdir");

    let r2 = lal::export(backend, "libcurl", None, Some("xenial"));
    assert!(r2.is_ok(), "could export latest libcurl into PWD");

    let gtest = Path::new(&env::current_dir().unwrap()).join("tests").join("gtest.tar.gz");
    assert!(gtest.is_file(), "gtest was copied correctly");

    let libcurl = Path::new(&env::current_dir().unwrap()).join("libcurl.tar.gz");
    assert!(libcurl.is_file(), "libcurl was copied correctly");

    // clean up
    fs::remove_file(&gtest).unwrap();
    fs::remove_file(&libcurl).unwrap();
}
