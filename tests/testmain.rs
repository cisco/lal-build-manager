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
    let tmprelative = Path::new(".").join("testtmp");
    if !tmprelative.is_dir() {
        fs::create_dir(&tmprelative).unwrap();
    }
    let tmp = fs::canonicalize(tmprelative).unwrap();

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

    no_publish_non_release_builds(&backend);
    info!("ok no_publish_non_release_builds heylib");

    let helloworlddir = testdir.join("helloworld");
    assert!(env::set_current_dir(&helloworlddir).is_ok());

    update_save(&backend);
    info!("ok update_save");

    verify_checks(&backend);
    info!("ok verify_checks");

    fetch_release_build_and_publish(&backend);
    info!("ok fetch_release_build_and_publish helloworld");

    remove_dependencies();
    info!("ok remove_dependencies");

    // back to tmpdir to test export and clean
    assert!(env::set_current_dir(&tmp).is_ok());
    export_check(&backend);
    info!("ok export_check");

    query_check(&backend);
    info!("ok query_check");

    clean_check();
    info!("ok clean_check");

    // finally test out some functionality regarding creating of new components
    // we just do this in the same temp directory as there's nothing there
    init_force();
    info!("ok init_force");

    has_config_and_manifest();
    info!("ok has_config_and_manifest");

    list_everything();
    info!("ok list_everything");

    change_envs();
    info!("ok change_envs");

    kill_manifest();
    info!("ok kill_manifest");

    // verify propagations by building prop-leaf -> prop-mid-X -> prop-base
    let propleaf = testdir.join("prop-leaf");
    assert!(env::set_current_dir(&propleaf).is_ok());
    fetch_release_build_and_publish(&backend);
    info!("ok fetch_release_build_and_publish prop-leaf");

    let propmid1 = testdir.join("prop-mid-1");
    assert!(env::set_current_dir(&propmid1).is_ok());
    fetch_release_build_and_publish(&backend);
    info!("ok fetch_release_build_and_publish prop-mid-1");

    let propmid2 = testdir.join("prop-mid-2");
    assert!(env::set_current_dir(&propmid2).is_ok());
    fetch_release_build_and_publish(&backend);
    info!("ok fetch_release_build_and_publish prop-mid-2");

    let propbase = testdir.join("prop-base");
    assert!(env::set_current_dir(&propbase).is_ok());
    fetch_release_build_and_publish(&backend);
    info!("ok fetch_release_build_and_publish prop-base");

    check_propagation("prop-leaf");
    info!("ok check_propagation prop-leaf -> prop-base");
}

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

fn remove_dependencies() {
    let mf = Manifest::read().unwrap();
    let xs = mf.dependencies.keys().cloned().collect::<Vec<_>>();
    let r = lal::remove(&mf, xs.clone(), false, false);
    assert!(r.is_ok(), "could lal rm all dependencies");

    let rs = lal::remove(&mf, xs, true, false);
    assert!(rs.is_ok(), "could lal rm all dependencies and save");

    // should be no dependencies now
    let mf2 = Manifest::read().unwrap();
    let xs2 = mf2.dependencies.keys().cloned().collect::<Vec<_>>();
    assert_eq!(xs2.len(), 0);

    mf.write().unwrap(); // save the old one again
}

fn change_envs() {
    let cfg = Config::read().unwrap();
    let mf = Manifest::read().unwrap();

    // no sticky flags set yet
    let sticky_none = StickyOptions::read().unwrap();
    assert_eq!(sticky_none.env, None);

    // update the container associated with the default env
    // (on CI we've already done this at test start => cheap)
    let container = cfg.get_container(mf.environment.clone()).unwrap();
    let ru = lal::env::update(&container, &mf.environment);
    assert!(ru.is_ok(), "env update succeeded");

    let rc = lal::env::set(&sticky_none, &cfg, "xenial");
    assert!(rc.is_ok(), "env set xenial succeeded");

    // we changed the sticky option with that
    let sticky_set = StickyOptions::read().unwrap();
    assert_eq!(sticky_set.env, Some("xenial".into()));

    let rc = lal::env::clear();
    assert!(rc.is_ok(), "env clear succeeded");

    // we cleared the stickies with that
    let sticky_clear = StickyOptions::read().unwrap();
    assert_eq!(sticky_clear.env, None);
}

fn kill_manifest() {
    let pwd = env::current_dir().unwrap();
    let manifest = Path::new(&pwd).join("manifest.json");
    if manifest.is_file() {
        fs::remove_file(&manifest).unwrap();
    }
    assert_eq!(manifest.is_file(), false);
}

fn list_everything() {
    let cfg = Config::read().unwrap();
    let mf = Manifest::read().unwrap();

    let re = lal::list::environments(&cfg);
    assert!(re.is_ok(), "list envs succeeded");

    let rdc = lal::list::dependencies(&mf, true);
    assert!(rdc.is_ok(), "list deps --core succeeded");
    let rd = lal::list::dependencies(&mf, false);
    assert!(rd.is_ok(), "list deps succeeded");

    let rc = lal::list::configurations(&mf.name, &mf);
    assert!(rc.is_ok(), "list configurations succeeded");

    let rb = lal::list::buildables(&mf);
    assert!(rb.is_ok(), "list buildables succeeded");
}

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

// Create manifest in a weird directory
fn init_force() {
    let cfg = Config::read().unwrap();

    let m1 = Manifest::read();
    assert!(m1.is_err(), "no manifest at this point");

    // Creates a manifest in the testtmp directory
    let m2 = lal::init(&cfg, false, "alpine");
    assert!(m2.is_ok(), "could init without force param");

    let m3 = lal::init(&cfg, true, "alpine");
    assert!(m3.is_ok(), "could re-init with force param");

    let m4 = lal::init(&cfg, false, "alpine");
    assert!(m4.is_err(), "could not re-init without force ");

    let m5 = lal::init(&cfg, true, "blah");
    assert!(m5.is_err(), "could not init without valid environment");
}

// Tests need to be run in a directory with a manifest
// and ~/.lal + config must exist
fn has_config_and_manifest() {
    let ldir = config_dir();
    assert!(ldir.is_dir(), "have laldir");

    let cfg = Config::read();
    chk::is_ok(cfg, "could read config");

    let manifest = Manifest::read();
    chk::is_ok(Manifest::read(), "could read manifest");

    // There is no INPUT yet, but we have no dependencies, so this should work:
    let r = lal::verify(&manifest.unwrap(), "xenial".into(), false);
    chk::is_ok(r, "could verify after install");
}

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

fn no_publish_non_release_builds<T: CachedBackend + Backend>(backend: &T) {
    let mf = Manifest::read().unwrap();
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("alpine".into()).unwrap();

    let artifact_dir = Path::new("./ARTIFACT");
    if artifact_dir.is_dir() {
        debug!("Deleting existing artifact dir");
        fs::remove_dir_all(&artifact_dir).unwrap();
    }

    let mut bopts = BuildOptions {
        name: None,
        configuration: Some("release".into()),
        container: container,
        release: false, // missing releaes bad
        version: Some("2".into()), // but have version
        sha: None,
        force: false,
        simple_verify: false,
    };
    let modes = ShellModes::default();
    let r = lal::build(&cfg, &mf, &bopts, "alpine".into(), modes.clone());
    assert!(r.is_ok(), "could build without non-release");

    let rp = lal::publish(&mf.name, backend);
    assert!(rp.is_err(), "could not publish non-release build");

    bopts.version = None; // missing version bad
    bopts.release = true; // but at least in release mode now

    let rb2 = lal::build(&cfg, &mf, &bopts, "alpine".into(), modes.clone());
    assert!(rb2.is_ok(), "could build in without version");

    let rp2 = lal::publish(&mf.name, backend);
    assert!(rp2.is_err(), "could not publish without version set");

}
// add dependencies to test tree
// NB: this currently shouldn't do anything as all deps are accounted for
// Thus if this changes test manifests, something is wrong..
fn update_save<T: CachedBackend + Backend>(backend: &T) {
    let mf1 = Manifest::read().unwrap();

    // update heylib --save
    let ri = lal::update(&mf1,
                         backend,
                         vec!["heylib".to_string()],
                         true,
                         false,
                         "alpine");
    chk::is_ok(ri, "could update heylib and save");

    // main deps (and re-read manifest to avoid overwriting devedps)
    let mf2 = Manifest::read().unwrap();
    let updates = vec![
        "heylib".to_string(),
        // TODO: more deps
    ];
    let ri = lal::update(&mf2, backend, updates, true, false, "alpine");
    chk::is_ok(ri, "could update and save");

    // verify update-all --save
    let mf3 = Manifest::read().unwrap();
    let ri = lal::update_all(&mf3, backend, true, false, "alpine");
    chk::is_ok(ri, "could update all and --save");

    // verify update-all --save --dev
    let mf4 = Manifest::read().unwrap();
    let ri = lal::update_all(&mf4, backend, false, true, "alpine");
    chk::is_ok(ri, "could update all and --save --dev");
}

fn verify_checks<T: CachedBackend + Backend>(backend: &T) {
    let mf = Manifest::read().unwrap();

    let rcore = lal::fetch(&mf, backend, true, "alpine");
    assert!(rcore.is_ok(), "install core succeeded");

    let r = lal::verify(&mf, "alpine".into(), false);
    assert!(r.is_ok(), "could verify after install");

    let renv1 = lal::verify(&mf, "xenial".into(), false);
    assert!(renv1.is_err(), "could not verify with wrong env");
    let renv2 = lal::verify(&mf, "xenial".into(), true);
    assert!(renv2.is_err(),
            "could not verify with wrong env - even with simple");

    let heylib = Path::new(&env::current_dir().unwrap()).join("INPUT").join("heylib");
    // clean folders and verify it fails
    fs::remove_dir_all(&heylib).unwrap();

    let r2 = lal::verify(&mf, "alpine".into(), false);
    assert!(r2.is_err(), "verify failed after fiddling");

    // fetch --core, resyncs with core deps (removes devDeps and other extraneous)
    let rcore = lal::fetch(&mf, backend, true, "alpine");
    assert!(rcore.is_ok(), "install core succeeded");
    assert!(heylib.is_dir(), "heylib was reinstalled from manifest");
    // TODO: add dev dep to verify it wasn't reinstalled here
    //assert!(!gtest.is_dir(), "gtest was was extraneous with --core => removed");

    // fetch --core also doesn't install else again
    let rcore2 = lal::fetch(&mf, backend, true, "alpine");
    assert!(rcore2.is_ok(), "install core succeeded 2");
    assert!(heylib.is_dir(), "heylib still there");
    //assert!(!gtest.is_dir(), "gtest was not reinstalled with --core");

    // and it is finally installed if we ask for non-core as well
    let rall = lal::fetch(&mf, backend, false, "alpine");
    assert!(rall.is_ok(), "install all succeeded");
    //assert!(gtest.is_dir(), "gtest is otherwise installed again");

    let r3 = lal::verify(&mf, "alpine", false);
    assert!(r3.is_ok(), "verify ok again");
}

fn run_scripts() {
    {
        Command::new("mkdir").arg("-p").arg(".lal/scripts").output().unwrap();
        let mut f = File::create("./.lal/scripts/subroutine").unwrap();
        write!(f, "main() {{ echo hi $1 $2 ;}}\n").unwrap();
        Command::new("chmod").arg("+x").arg(".lal/scripts/subroutine").output().unwrap();
    }
    let cfg = Config::read().unwrap();
    let container = cfg.get_container("alpine".into()).unwrap();
    let modes = ShellModes::default();
    let r = lal::script(&cfg,
                        &container,
                        "subroutine",
                        vec!["there", "mr"],
                        &modes,
                        false);
    assert!(r.is_ok(), "could run subroutine script");
}

fn check_propagation(leaf: &str) {
    let mf = Manifest::read().unwrap();

    let lf = Lockfile::default().set_name(&mf.name).populate_from_input().unwrap();
    if let Ok(res) = lal::propagate::compute(&lf, leaf) {
        assert_eq!(res.stages.len(), 2);
        // first stage
        assert_eq!(res.stages[0].updates.len(), 2); // must update both mid points
        assert_eq!(res.stages[0].updates[0].dependencies, vec!["prop-leaf"]);
        assert_eq!(res.stages[0].updates[1].dependencies, vec!["prop-leaf"]);
        assert_eq!(res.stages[0].updates[0].repo, "prop-mid-1");
        assert_eq!(res.stages[0].updates[1].repo, "prop-mid-2");
        // second stage
        assert_eq!(res.stages[1].updates.len(), 1); // must update base
        assert_eq!(res.stages[1].updates[0].dependencies, vec!["prop-mid-1", "prop-mid-2"]);
        assert_eq!(res.stages[1].updates[0].repo, "prop-base");
    } else {
        assert!(false, "could propagate leaf to {}", mf.name);
    }

    let rpj = lal::propagate::print(&mf, leaf, true);
    assert!(rpj.is_ok(), "could print propagate json to stdout");
    let rp = lal::propagate::print(&mf, leaf, false);
    assert!(rp.is_ok(), "could print propagate to stdout");

    // print tree for extra coverage of bigger trees
    let rs = lal::status(&mf, true, true, true);
    assert!(rs.is_ok(), "could print status of propagation root");
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

    let rl = lal::query(backend, Some("alpine"), "hello", true);
    assert!(rl.is_ok(), "could query latest for hello");

}
