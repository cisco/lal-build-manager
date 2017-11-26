extern crate lal;

#[macro_use]
extern crate log;
extern crate loggerv;
extern crate walkdir;

use std::env;
use std::path::Path;
use std::fs::{self, File};
use std::process::Command;
use std::io::prelude::*;
use walkdir::WalkDir;

use loggerv::init_with_verbosity;
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
    // print debug output and greater from lal during tests
    init_with_verbosity(2).unwrap();

    // Do all lal tests in a subdir as it messes with the manifest
    let tmp = fs::canonicalize(Path::new(".").join("testtmp")).unwrap();
    if !tmp.is_dir() {
        fs::create_dir(&tmp).unwrap();
    }
    // Ensure we are can do everything in there before continuing
    assert!(env::set_current_dir(tmp.clone()).is_ok());
    // dump config and artifacts under the current temp directory
    env::set_var("LAL_CONFIG_HOME", env::current_dir().unwrap());

    info!("# lal tests");

    // Set up a fresh LAL_CONFIG_HOME and reconfigure
    kill_laldir();
    info!("ok kill_laldir");
    let backend = configure_yes();
    info!("ok configure_yes");

    let testdir = fs::canonicalize(Path::new("..").join("tests")).unwrap();


    // Test basic build functionality with heylib component
    let heylibdir = testdir.join("heylib");
    assert!(env::set_current_dir(heylibdir).is_ok());

    kill_input();
    info!("ok kill_input");

    shell_echo();
    info!("ok shell_echo");

    shell_permissions();
    info!("ok shell_permissions");

    build_and_stash_update_self(&backend);
    info!("ok build_and_stash_update_self");

    status_on_experimentals();
    info!("ok status_on_experimentals");

    run_scripts();
    info!("ok run_scripts");

    fetch_release_build_and_publish(&backend);
    info!("ok fetch_release_build_and_publish heylib");

    let helloworlddir = testdir.join("helloworld");
    assert!(env::set_current_dir(&helloworlddir).is_ok());

    fetch_release_build_and_publish(&backend);
    info!("ok fetch_release_build_and_publish helloworld");

    // back to tmpdir to test export and clean
    assert!(env::set_current_dir(&tmp).is_ok());
    export_check(&backend);
    info!("ok export_check");

    query_check(&backend);
    info!("ok query_check");

    clean_check();
    info!("ok clean_check");

    // TODO: verify stash + update

/*
    kill_manifest();
    info!("ok kill_manifest");

    init_force();
    info!("ok init_force");

    has_config_and_manifest();
    info!("ok has_config_and_manifest");
    // assume we have manifest and config after this point

    update_save(&backend);
    info!("ok update_save");

    verify_checks(&backend);
    info!("ok verify_checks");
*/
}
// Start from scratch
fn kill_laldir() {
    let ldir = config_dir();
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
/*fn kill_manifest() {
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
}*/

// Create config
fn configure_yes() -> LocalBackend {
    let config = Config::read();
    assert!(config.is_err(), "no config at this point");

    let r = lal::configure(true, false, "../configs/demo.json");
    assert!(r.is_ok(), "configure succeeded");

    let cfg = Config::read();
    assert!(cfg.is_ok(), "config exists now");

    let cfgu = cfg.unwrap();

    match &cfgu.backend {
        &BackendConfiguration::Local(ref local_cfg) => {
            LocalBackend::new(&local_cfg, &cfgu.cache)
        }
        _ => unreachable!() // demo.json uses local backend
    }
}
/*
// Create manifest
fn init_force() {
    let cfg = Config::read().unwrap();

    let m1 = Manifest::read();
    assert!(m1.is_err(), "no manifest at this point");

    // Creates a manifest in the testtmp directory
    let m2 = lal::init(&cfg, false, "rust");
    assert!(m2.is_ok(), "could init without force param");

    let m3 = lal::init(&cfg, true, "rust");
    assert!(m3.is_ok(), "could re-init with force param");

    let m4 = lal::init(&cfg, false, "rust");
    assert!(m4.is_err(), "could not re-init without force ");

    let m5 = lal::init(&cfg, true, "blah");
    assert!(m5.is_err(), "could not init without valid environment");
}*/

// Tests need to be run in a directory with a manifest
// and ~/.lal + config must exist
/*fn has_config_and_manifest() {
    let ldir = config_dir();
    assert!(ldir.is_dir(), "have laldir");

    let cfg = Config::read();
    chk::is_ok(cfg, "could read config");

    let manifest = Manifest::read();
    chk::is_ok(Manifest::read(), "could read manifest");

    // There is no INPUT yet, but we have no dependencies, so this should work:
    let r = lal::verify(&manifest.unwrap(), "xenial".into(), false);
    chk::is_ok(r, "could verify after install");
}*/

/*
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
    let updates = vec![
        "libyaml".to_string(),
        "yajl".to_string(),
        "libwebsockets".to_string(),
    ];
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

    let renv1 = lal::verify(&mf, "zesty".into(), false);
    assert!(renv1.is_err(), "could not verify with wrong env");
    let renv2 = lal::verify(&mf, "zesty".into(), true);
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
}*/

// Shell tests
fn shell_echo() {
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("alpine".into()).unwrap();
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
    let container = cfg.get_container("alpine".into()).unwrap();
    let modes = ShellModes::default();
    let r = lal::docker_run(&cfg,
                            &container,
                            vec!["touch".to_string(), "README.md".to_string()],
                            &DockerRunFlags::default(),
                            &modes);
    assert!(r.is_ok(), "could touch files in container");
}

fn build_and_stash_update_self<T: CachedBackend + Backend>(backend: &T) {
    let mf = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("alpine".into()).unwrap();

    // we'll try with various build options further down with various deps
    let mut bopts = BuildOptions {
        name: Some("heylib".into()),
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
    let r = lal::build(&cfg, &mf, &bopts, "alpine".into(), modes.clone());
    if let Err(e) = r {
        println!("error from build: {:?}", e);
        assert!(false, "could perform an alpine build");
    }

    // lal stash blah
    let rs = lal::stash(backend, &mf, "blah");
    assert!(rs.is_ok(), "could stash lal build artifact");

    // lal update heylib=blah
    let ru = lal::update(&mf,
                         backend,
                         vec!["heylib=blah".to_string()],
                         false,
                         false,
                         "garbage"); // env not relevant for stash
    chk::is_ok(ru, "could update heylib from stash");

    // basic build won't work now without simple verify
    let r1 = lal::build(&cfg, &mf, &bopts, "alpine".into(), modes.clone());
    assert!(r1.is_err(), "could not verify a new alpine build");
    if let Err(CliError::NonGlobalDependencies(nonglob)) = r1 {
        assert_eq!(nonglob, "heylib");
    } else {
        println!("actual r1 was {:?}", r1);
        assert!(false);
    }

    bopts.simple_verify = true;
    let r2 = lal::build(&cfg, &mf, &bopts, "alpine".into(), modes.clone());
    assert!(r2.is_ok(), "can build with stashed deps with simple verify");


    // force will also work - even with stashed deps from wrong env
    let renv = lal::build(&cfg, &mf, &bopts, "xenial".into(), modes.clone());
    assert!(renv.is_err(),
            "cannot build with simple verify when wrong env");
    if let Err(CliError::EnvironmentMismatch(_, compenv)) = renv {
        assert_eq!(compenv, "alpine"); // expected complaints about xenial env
    } else {
        println!("actual renv was {:?}", renv);
        assert!(false);
    }

    // settings that reflect lal build -f
    bopts.simple_verify = false;
    bopts.force = true;
    let renv2 = lal::build(&cfg, &mf, &bopts, "xenial".into(), modes.clone());
    assert!(renv2.is_ok(), "could force build in different env");

    // additionally do a build with printonly
    let all_modes = ShellModes {
        printonly: true,
        x11_forwarding: true,
        host_networking: true,
        env_vars: vec![],
    };
    let printbuild = lal::build(&cfg, &mf, &bopts, "alpine".into(), all_modes);
    // TODO: verify output!
    assert!(printbuild.is_ok(), "saw docker run print with X11 mounts");
}


fn fetch_release_build_and_publish<T: CachedBackend + Backend>(backend: &T) {
    let mf = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("alpine".into()).unwrap();

    let rcore = lal::fetch(&mf, backend, true, "alpine");
    assert!(rcore.is_ok(), "install core succeeded");

    // we'll try with various build options further down with various deps
    let bopts = BuildOptions {
        name: None,
        configuration: Some("release".into()),
        container: container,
        release: true,
        version: Some("1".into()), // want to publish version 1 for later
        sha: None,
        force: false,
        simple_verify: false,
    };
    let modes = ShellModes::default();
    let r = lal::build(&cfg, &mf, &bopts, "alpine".into(), modes.clone());
    assert!(r.is_ok(), "could build in release");

    let rp = lal::publish(&mf.name, backend);
    assert!(rp.is_ok(), "could publish");
}


/*fn build_stash_and_update_from_stash() {
    {
        let mut f = File::create("./BUILD").unwrap();
        write!(f, "#!/bin/bash\nset -e\nwhich rustc\necho hi > test.txt\n").unwrap();
        Command::new("chmod").arg("+x").arg("BUILD").output().unwrap();
    } // scope ensures file is not busy before lal::build

}*/

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
    let r = lal::script(&cfg,
                        &container,
                        "subroutine",
                        vec!["there", "mr"],
                        &modes,
                        false);
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

#[cfg(feature = "upgrade")]
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
    let tmp = Path::new(".").join("blah");
    if !tmp.is_dir() {
        fs::create_dir(&tmp).unwrap();
    }
    let r = lal::export(backend, "heylib=1", Some("blah"), Some("alpine"));
    assert!(r.is_ok(), "could export heylib=1 into subdir");

    let r2 = lal::export(backend, "hello", None, Some("alpine"));
    assert!(r2.is_ok(), "could export latest hello into PWD");

    let heylib = Path::new(".").join("blah").join("heylib.tar.gz");
    assert!(heylib.is_file(), "heylib was copied correctly");

    let hello = Path::new(".").join("hello.tar.gz");
    assert!(hello.is_file(), "hello was copied correctly");

    // TODO: verify we can untar and execute hello binary and grep output after #15
}

fn query_check<T: Backend>(backend: &T) {
    let r = lal::query(backend, Some("alpine"), "hello", false);
    assert!(r.is_ok(), "could query for hello");

}
