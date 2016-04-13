#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;

extern crate lal;
use lal::{LalResult, Config, Manifest};

use clap::{Arg, App, AppSettings, SubCommand};
use std::process;

fn result_exit<T>(name: &str, x: LalResult<T>) {
    let _ = x.map_err(|e| {
        println!(""); // add a separator
        error!("{} error: {}", name, e);
        process::exit(1);
    });
    process::exit(0);
}

fn main() {
    let args = App::new("lal")
        .version(crate_version!())
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .about("lal dependency manager")
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Use verbose output"))
        .subcommand(SubCommand::with_name("install")
            .about("Installs dependencies listed in the manifest into INPUT")
            .arg(Arg::with_name("components")
                .help("Installs specific component=version pairs")
                .multiple(true))
            .arg(Arg::with_name("dev")
                .long("dev")
                .short("d")
                .help("Additionally install devDependencies")
                .conflicts_with("components"))
            .arg(Arg::with_name("save")
                .short("S")
                .long("save")
                .requires("components")
                .conflicts_with("savedev")
                .help("Save installed versions in dependencies in the manifest"))
            .arg(Arg::with_name("savedev")
                .short("D")
                .long("save-dev")
                .requires("components")
                .conflicts_with("save")
                .help("Save installed versions in devDependencies in the manifest")))
        .subcommand(SubCommand::with_name("uninstall")
            .about("Uninstalls specific dependencies from INPUT")
            .arg(Arg::with_name("components")
                .help("Installs specific component=version pairs")
                .required(true) // unlike install which works without components
                .multiple(true))
            .arg(Arg::with_name("save")
                .short("S")
                .long("save")
                .conflicts_with("savedev")
                .help("Save removal of dependencies in the manifest"))
            .arg(Arg::with_name("savedev")
                .short("D")
                .long("save-dev")
                .conflicts_with("save")
                .help("Save removal of devDependencies in the manifest")))
        .subcommand(SubCommand::with_name("build")
            .about("Runs BUILD script in current directory in the configured container")
            .arg(Arg::with_name("component")
                .help("Build a specific component (if other than the main manifest component)"))
            .arg(Arg::with_name("configuration")
                .long("config")
                .short("c")
                .takes_value(true)
                .help("Build using a specific configuration (else will use defaultConfig)"))
            .arg(Arg::with_name("strict")
                .long("strict")
                .short("s")
                .conflicts_with("release") // release is always strict
                .help("Fail build if verify fails"))
            .arg(Arg::with_name("release")
                .long("release")
                .short("r")
                .help("Create release output for artifactory"))
            .arg(Arg::with_name("with-version")
                .long("with-version")
                .takes_value(true)
                .requires("release")
                .help("Configure lockfiles for a release with an explicit new version")))
        .subcommand(SubCommand::with_name("stash")
            .about("Stashes current build OUTPUT in cache for later reuse")
            .arg(Arg::with_name("name")
                .required(true)
                .help("Name used for current build")))
        .subcommand(SubCommand::with_name("verify").about("runs verify script"))
        .subcommand(SubCommand::with_name("configure")
            .about("configures lal")
            .arg(Arg::with_name("yes")
                .short("y")
                .long("yes")
                .help("Assume default without prompting")))
        .subcommand(SubCommand::with_name("status")
            .about("Prints current dependencies and their status"))
        .subcommand(SubCommand::with_name("init")
            .about("Create a manifest file in the current directory")
            .arg(Arg::with_name("force")
                .short("f")
                .help("overwrites manifest if necessary")))
        .subcommand(SubCommand::with_name("shell")
            .about("Enters the configured container mounting the current directory"))
        .subcommand(SubCommand::with_name("upgrade")
            .about("Checks for a new version of lal manually"))
        .get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::init_with_verbosity(args.occurrences_of("verbose") + 1).unwrap();

    // Allow lal configure without assumptions
    if let Some(a) = args.subcommand_matches("configure") {
        result_exit("configure", lal::configure(!a.is_present("yes"), true));
    }

    // Force config to exists before allowing remaining actions
    let config = Config::read()
        .map_err(|e| {
            error!("Configuration error: {}", e);
            println!("Ensure you have run `lal configure` and that ~/.lal/lalrc is valid json");
            process::exit(1);
        })
        .unwrap();

    // Allow lal upgrade without manifest
    if let Some(_) = args.subcommand_matches("upgrade") {
        result_exit("upgrade", lal::upgrade_check(false)); // explicit, verbose check
    }
    // Timed daily, silent upgrade check (if not using upgrade)
    if args.subcommand_name() != Some("upgrade") && config.update_check_time() {
        debug!("Performing daily upgrade check");
        // silent dry-run
        let _ = lal::upgrade_check(false).map_err(|e| {
            error!("Daily update check failed: {}", e);
            // don't halt here if this ever happens as it could break it for users
        });
        let _ = config.clone().performed_update().map_err(|e| {
            error!("Daily update check updating lastUpdate failed: {}", e);
            // Ditto
        });
        debug!("Upgrade check done - continuing to requested operation\n");
    }

    // Allow lal init without manifest existing
    if let Some(a) = args.subcommand_matches("init") {
        result_exit("init", lal::init(a.is_present("force")));
    }

    // Force manifest to exist before allowing remaining actions
    let manifest = Manifest::read()
        .map_err(|e| {
            error!("Manifest error: {}", e);
            println!("Ensure you have run `lal init` and that manifest.json is valid json");
            process::exit(1);
        })
        .unwrap();

    // Remaining actions - assume Manifest and Config
    if let Some(a) = args.subcommand_matches("install") {
        let res = if a.is_present("components") {
            let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
            lal::install(manifest,
                         config,
                         xs,
                         a.is_present("save"),
                         a.is_present("savedev"))

        } else {
            lal::install_all(manifest, config, a.is_present("dev"))
        };
        result_exit("install", res);
    } else if let Some(a) = args.subcommand_matches("uninstall") {
        let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
        let res = lal::uninstall(manifest, xs, a.is_present("save"), a.is_present("savedev"));
        result_exit("uninstall", res);
    } else if let Some(a) = args.subcommand_matches("build") {
        let res = lal::build(&config,
                             &manifest,
                             a.value_of("component"),
                             a.value_of("configuration"),
                             a.is_present("release"),
                             a.value_of("with-version"),
                             a.is_present("strict"));
        result_exit("build", res);
    } else if let Some(_) = args.subcommand_matches("shell") {
        result_exit("shell", lal::shell(&config));
    } else if let Some(_) = args.subcommand_matches("verify") {
        result_exit("verify", lal::verify(manifest));
    } else if let Some(_) = args.subcommand_matches("status") {
        result_exit("status", lal::status(manifest));
    } else if let Some(a) = args.subcommand_matches("stash") {
        result_exit("stash",
                    lal::stash(config, manifest, a.value_of("name").unwrap()));
    }

    unreachable!("Subcommand valid, but not implemented");
}
