#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;

extern crate lal;
use lal::*;
use lal::init::Manifest;

use clap::{Arg, App, AppSettings, SubCommand};

use std::process;
use lal::errors::LalResult;

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
            .about("installs dependencies")
            .arg(Arg::with_name("components")
                .help("Installs specific component=version pairs")
                .multiple(true))
            .arg(Arg::with_name("dev")
                .long("dev")
                .short("d")
                .help("Install devDependencies as well")
                .conflicts_with("components"))
            .arg(Arg::with_name("save")
                .short("S")
                .long("save")
                .requires("components")
                .conflicts_with("savedev")
                .help("Install also updates dependencies in the manifest"))
            .arg(Arg::with_name("savedev")
                .short("D")
                .long("save-dev")
                .requires("components")
                .conflicts_with("save")
                .help("Install also updates devDependencies in the manifest")))
        .subcommand(SubCommand::with_name("build")
            .about("runs build script")
            .arg(Arg::with_name("component").help("Build a specific component"))
            .arg(Arg::with_name("name").help("build a specific component"))
            .arg(Arg::with_name("release")
                .long("release")
                .short("r")
                .help("Create release output for artifactory"))
            .arg(Arg::from_usage("--with-version [version] 'Configure lockfiles for a \
                                  release with an explicit new version'")))
        .subcommand(SubCommand::with_name("stash")
            .about("stashes current build OUTPUT in cache for later reuse")
            .arg(Arg::with_name("name")
                .required(true)
                .help("name used for current build")))
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
        .get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::init_with_verbosity(args.occurrences_of("verbose") + 1).unwrap();

    // Allow lal configure without assumptions
    if let Some(a) = args.subcommand_matches("configure") {
        result_exit("configure",
                    configure::configure(!a.is_present("yes"), true));
    }

    // Force config to exists before allowing remaining actions
    let config = configure::current_config()
        .map_err(|e| {
            error!("Configuration error: {}", e);
            println!("Ensure you have run `lal configure` and that ~/.lal/lalrc is valid json");
            process::exit(1);
        })
        .unwrap();

    // Allow lal init without manifest existing
    if let Some(a) = args.subcommand_matches("init") {
        result_exit("init", init::init(a.is_present("force")));
    }

    // Force manifest to exist before allowing remaining actions
    let manifest = Manifest::read()
        .map_err(|e| {
            error!("Manifest error: {}", e);
            println!("Ensure you have run `lal init` and that manifest.json is valid json");
            process::exit(1);
        })
        .unwrap();


    // Remaining actions
    if let Some(a) = args.subcommand_matches("install") {
        let res = if a.is_present("components") {
            let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
            install::install(manifest,
                             config,
                             xs,
                             a.is_present("save"),
                             a.is_present("savedev"))

        } else {
            install::install_all(manifest, config, a.is_present("dev"))
        };
        result_exit("install", res);
    } else if let Some(a) = args.subcommand_matches("build") {
        let res = build::build(&config,
                               &manifest,
                               a.value_of("component"),
                               a.is_present("release"),
                               a.value_of("with-version"));
        result_exit("build", res);
    } else if let Some(_) = args.subcommand_matches("shell") {
        result_exit("shell", shell::shell(&config));
    } else if let Some(_) = args.subcommand_matches("verify") {
        result_exit("verify", verify::verify(manifest));
    } else if let Some(_) = args.subcommand_matches("status") {
        result_exit("status", status::status(manifest));
    } else if let Some(a) = args.subcommand_matches("stash") {
        result_exit("stash",
                    cache::stash(config, manifest, a.value_of("name").unwrap()));
    }

    unreachable!("Subcommand valid, but not implemented");
}
