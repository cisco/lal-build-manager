extern crate clap;
extern crate curl;
extern crate rustc_serialize;

use clap::{Arg, App, SubCommand};

pub mod build;
pub mod install;
pub mod configure;
pub mod verify;
pub mod init;

fn main() {
    let args = App::new("lal")
                   .version("1.2.3")
                   .about("lal dependency manager")
                   .arg(Arg::with_name("verbose")
                            .short("v")
                            .help("Use verbose output"))
                   .subcommand(SubCommand::with_name("install")
                                   .about("installs dependencies")
                                   .arg(Arg::with_name("components")
                                            .help("Installs specific component=version pairs")
                                            .multiple(true))
                                   .arg(Arg::with_name("dev")
                                            .help("Install devDependencies as well")
                                            .conflicts_with("components"))
                                   .arg(Arg::with_name("save")
                                            .short("s")
                                            .long("save")
                                            .requires("components")
                                            .help("Install also updates manifest")))
                   .subcommand(SubCommand::with_name("build")
                                   .about("runs build script")
                                   .arg(Arg::with_name("name").help("build a specific component")))
                   .subcommand(SubCommand::with_name("stash")
                                   .about("stashes current OUTPUT in cache")
                                   .arg(Arg::with_name("name")
                                            .required(true)
                                            .help("name used for current build")))
                   .subcommand(SubCommand::with_name("verify").about("runs verify script"))
                   .subcommand(SubCommand::with_name("configure").about("configures lal"))
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

    // Configuration of lal first.
    if let Some(_) = args.subcommand_matches("configure") {
        let _ = configure::configure();
        return;
    }
    // Assume config exists before allowing other actions
    let config = configure::current_config().unwrap();

    if let Some(a) = args.subcommand_matches("install") {
        if a.is_present("components") {
            let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
            return install::install(xs, a.is_present("save"));
        } else {
            return install::install_all();
        }
    }
    if let Some(_) = args.subcommand_matches("build") {
        return build::build();
    }
    if let Some(_) = args.subcommand_matches("verify") {
        return verify::verify();
    }
    if let Some(a) = args.subcommand_matches("init") {
        let _ = init::init(a.is_present("force"));
        return;
    }
    println!("print help");
}
