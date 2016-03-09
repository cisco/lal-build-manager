extern crate lal;

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::{Path, PathBuf};
    use std::fs;

    use lal::{configure, install, verify, init, shell};

    fn lal_dir() -> PathBuf {
        let home = env::home_dir().unwrap();
        Path::new(&home).join(".lal/")
    }

    // Tests need to be run in a directory with a manifest
    // and ~/.lal + lalrc must exist
    #[test]
    fn sanity() {
        let ldir = lal_dir();
        assert_eq!(ldir.is_dir(), true);

        let cfg = configure::current_config();
        assert_eq!(cfg.is_ok(), true);

        let manifest = init::read_manifest();
        assert_eq!(manifest.is_ok(), true);
    }
    
    // These tests screw with the other tests which are also reading lalrc
    // Can run them from scratch with `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn hide_lalrc() {
        let ldir = lal_dir();
        if ldir.is_dir() {
            fs::remove_dir_all(&ldir).unwrap();
        }
        assert_eq!(ldir.is_dir(), false);
    }

    #[test]
    #[ignore]
    fn configure_without_lalrc() {
        let r = configure::configure(false);
        assert_eq!(r.is_ok(), true);
        let cfg = configure::current_config();
        assert_eq!(cfg.is_ok(), true);
    }

    #[test]
    #[ignore]
    fn fails_on_missing_dir() {
        // Can't really run this consistenly unless create an order of tests
        // if they're all in separate files all messing with INPUT it's silly
        let manifest = init::read_manifest();
        assert_eq!(manifest.is_ok(), true);
        let mf = manifest.unwrap();
        let config = configure::current_config();
        assert_eq!(config.is_ok(), true);
        let cfg = config.unwrap();

        let r = verify::verify();
        assert_eq!(r.is_err(), true);
        let ri = install::install_all(mf, cfg, false);
        assert_eq!(ri.is_ok(), true);

        let r = verify::verify();
        assert_eq!(r.is_ok(), true);
    }

        fn component_dir(name: &str) -> PathBuf {
        Path::new(&env::current_dir().unwrap()).join("INPUT").join(&name).join("ncp.amd64")
    }

    #[test]
    #[ignore]
    fn blank_state() {
        let input = Path::new(&env::current_dir().unwrap()).join("INPUT");
        if input.is_dir() {
            fs::remove_dir_all(&input).unwrap();
        }
        assert_eq!(input.is_dir(), false);
    }

    #[test]
    fn install_basic() {
        // This is ok if sanity test passed.
        // Could perhaps mock this..
        let mf = init::read_manifest().unwrap();
        let cfg = configure::current_config().unwrap();

        // Check that install installs stuff from manifest
        let r1 = install::install(mf.clone(), cfg.clone(), vec!["gtest"], false, false);
        assert_eq!(r1.is_ok(), true);
        assert_eq!(component_dir("gtest").is_dir(), true);
        let r2 = install::install(mf.clone(), cfg.clone(), vec!["libyaml"], false, false);
        assert_eq!(r2.is_ok(), true);
        assert_eq!(component_dir("libyaml").is_dir(), true);
    }

    // Shell tests
    #[test]
    fn can_run_commands() {
        let cfg = configure::current_config().unwrap();
        let r = shell::docker_run(&cfg, vec!["echo", "echo from docker"], false);
        assert_eq!(r.is_ok(), true);
    }
    #[test]
    fn can_touch_mounted_files() {
        let cfg = configure::current_config().unwrap();
        let r = shell::docker_run(&cfg, vec!["touch", "README.md"], false);
        assert_eq!(r.is_ok(), true);
    }

}
