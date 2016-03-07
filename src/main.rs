#[macro_use]
extern crate clap;
extern crate curl;
extern crate rustc_serialize;
extern crate regex;
extern crate tar;
extern crate flate2;
#[macro_use]
extern crate log;
extern crate loggerv;

use clap::{Arg, App, SubCommand};

pub mod errors;
pub mod configure;
pub mod init;
pub mod shell;
pub mod build;
pub mod install;
pub mod verify;
pub mod cache;

fn main() {
    use std::process;
    let args = App::new("lal")
                   .version(crate_version!())
                   .setting(clap::AppSettings::GlobalVersion)
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
                                            .help("Install also updates dependencies in the \
                                                   manifest"))
                                   .arg(Arg::with_name("savedev")
                                            .short("D")
                                            .long("save-dev")
                                            .requires("components")
                                            .conflicts_with("save")
                                            .help("Install also updates devDependencies in the \
                                                   manifest")))
                   .subcommand(SubCommand::with_name("build")
                                   .about("runs build script")
                                   .arg(Arg::with_name("name").help("build a specific component")))
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
                                   .about("Enters the configured container mounting the current \
                                           directory"))
                   .get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::init_with_verbosity(args.occurrences_of("verbose") + 1).unwrap();

    // Configuration of lal first.
    if let Some(a) = args.subcommand_matches("configure") {
        let _ = configure::configure(!a.is_present("yes")).map_err(|e| {
            error!("Failed to configure {}", e);
            process::exit(1);
        });
        process::exit(0);
    }
    // Assume config exists before allowing other actions
    let config = configure::current_config()
                     .map_err(|e| {
                         error!("Configuration error: {}", e);
                         println!("Ensure you have run `lal configure` and that ~/.lal/lalrc is \
                                   valid json");
                         process::exit(1);
                     })
                     .unwrap();

    if let Some(a) = args.subcommand_matches("init") {
        let _ = init::init(a.is_present("force")).map_err(|e| {
            error!("Init error: {}", e);
            process::exit(1);
        });
        process::exit(0);
    }

    // The other commands require a valid manifest
    let manifest = init::read_manifest()
                       .map_err(|e| {
                           error!("Manifest error: {}", e);
                           println!("Ensure you have run `lal init` and that manifest.json is \
                                     valid json");
                           process::exit(1);
                       })
                       .unwrap();


    if let Some(a) = args.subcommand_matches("install") {
        if a.is_present("components") {
            let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
            return install::install(manifest,
                                    config,
                                    xs,
                                    a.is_present("save"),
                                    a.is_present("savedev"));
        } else {
            return install::install_all(manifest, config, a.is_present("dev"));
        }
    }

    if let Some(_) = args.subcommand_matches("build") {
        return build::build(&config);
    }
    if let Some(_) = args.subcommand_matches("shell") {
        return shell::shell(&config);
    }
    if let Some(_) = args.subcommand_matches("verify") {
        let res = verify::verify().map_err(|e| error!("{}", e));
        process::exit(if res.is_ok() {
            0
        } else {
            1
        });
    }

    println!("{}", args.usage());
}
