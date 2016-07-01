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

fn is_integer(v: String) -> Result<(), String> {
    if v.parse::<u32>().is_ok() {
        return Ok(());
    }
    Err(format!("{} is not an integer", v))
}
fn main() {
    let args = App::new("lal")
        .version(crate_version!())
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .global_settings(&[AppSettings::ColoredHelp])
        .about("lal dependency manager")
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Use verbose output"))
        .subcommand(SubCommand::with_name("fetch")
            .about("Fetch dependencies listed in the manifest into INPUT")
            .arg(Arg::with_name("core")
                .long("core")
                .short("c")
                .help("Only fetch core dependencies")))
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
                .help("Configure lockfiles for a release with an explicit new version"))
            .arg(Arg::with_name("print")
                .long("print-only")
                .conflicts_with("release")
                .help("Only print the docker run command and exit")))
        .subcommand(SubCommand::with_name("update")
            .about("Update arbitrary dependencies into INPUT")
            .alias("up")
            .arg(Arg::with_name("components")
                .help("The specific component=version pairs to update")
                .required(true)
                .multiple(true))
            .arg(Arg::with_name("save")
                .short("S")
                .long("save")
                .conflicts_with("savedev")
                .help("Save updated versions in dependencies in the manifest"))
            .arg(Arg::with_name("savedev")
                .short("D")
                .long("save-dev")
                .conflicts_with("save")
                .help("Save updated versions in devDependencies in the manifest")))
        .subcommand(SubCommand::with_name("verify").about("verify consistency of INPUT"))
        .subcommand(SubCommand::with_name("status")
            .alias("ls")
            .arg(Arg::with_name("full")
                .short("f")
                .long("full")
                .help("Print the full dependency tree"))
            .about("Prints current dependencies and their status"))
        .subcommand(SubCommand::with_name("shell")
            .about("Enters the configured container mounting the current directory")
            .alias("sh")
            .arg(Arg::with_name("privileged")
                .short("p")
                .long("privileged")
                .help("Run docker in privileged mode"))
            .arg(Arg::with_name("print")
                .long("print-only")
                .help("Only print the docker run command and exit"))
            .setting(AppSettings::TrailingVarArg)
            .arg(Arg::with_name("cmd").multiple(true)))
        .subcommand(SubCommand::with_name("script")
            .about("Runs scripts from .lal/scripts in the configured container")
            .alias("run")
            .arg(Arg::with_name("script")
                .help("Name of the script file to be run")
                .required(true))
            .arg(Arg::with_name("privileged")
                .short("p")
                .long("privileged")
                .help("Run docker in privileged mode"))
            .setting(AppSettings::TrailingVarArg)
            .arg(Arg::with_name("parameters")
                .multiple(true)
                .help("Parameters to pass on to the script")))
        .subcommand(SubCommand::with_name("init")
            .about("Create a manifest file in the current directory")
            .arg(Arg::with_name("force")
                .short("f")
                .help("overwrites manifest if necessary")))
        .subcommand(SubCommand::with_name("configure")
            .about("configures lal")
            .arg(Arg::with_name("yes")
                .short("y")
                .long("yes")
                .help("Assume default without prompting")))
        .subcommand(SubCommand::with_name("export")
            .about("Fetch a raw tarball from artifactory")
            .arg(Arg::with_name("component")
                .help("The component to export")
                .required(true))
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Output directory to save to")))
        .subcommand(SubCommand::with_name("stash")
            .about("Stashes current build OUTPUT in cache for later reuse")
            .alias("save")
            .arg(Arg::with_name("name")
                .required(true)
                .help("Name used for current build")))
        .subcommand(SubCommand::with_name("upgrade")
            .about("Checks for a new version of lal manually"))
        .subcommand(SubCommand::with_name("remove")
            .alias("rm")
            .about("Remove specific dependencies from INPUT")
            .arg(Arg::with_name("components")
                .help("Remove specific component=version pairs")
                .required(true)
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
        .subcommand(SubCommand::with_name("clean")
            .about("Clean old artifacts in the cache directory to save space")
            .arg(Arg::with_name("days")
                .short("d")
                .long("days")
                .takes_value(true)
                .default_value("14")
                .validator(is_integer)
                .help("Number of days to serve as cutoff")))
        .subcommand(SubCommand::with_name("list-components")
            //.hidden(true) want this
            .about("list components that can be used with lal build"))
        .get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::init_with_verbosity(args.occurrences_of("verbose") + 1).unwrap();

    // Allow lal configure without assumptions
    if let Some(a) = args.subcommand_matches("configure") {
        result_exit("configure",
                    lal::configure(!a.is_present("yes"), true, None));
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
        result_exit("upgrade", lal::upgrade_check(&config, false)); // explicit, verbose check
    }
    // Timed daily, silent upgrade check (if not using upgrade)
    if args.subcommand_name() != Some("upgrade") && config.upgrade_check_time() {
        debug!("Performing daily upgrade check");
        // silent dry-run
        let _ = lal::upgrade_check(&config, false).map_err(|e| {
            error!("Daily upgrade check failed: {}", e);
            // don't halt here if this ever happens as it could break it for users
        });
        let _ = config.clone().performed_upgrade().map_err(|e| {
            error!("Daily upgrade check updating lastUpgrade failed: {}", e);
            // Ditto
        });
        debug!("Upgrade check done - continuing to requested operation\n");
    }

    // Allow lal init / clean without manifest existing in PWD
    if let Some(a) = args.subcommand_matches("init") {
        result_exit("init", lal::init(a.is_present("force")));
    } else if let Some(a) = args.subcommand_matches("clean") {
        let days = a.value_of("days").unwrap().parse().unwrap();
        result_exit("clean", lal::clean(&config, days));
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
    if let Some(a) = args.subcommand_matches("update") {
        let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
        let res = lal::update(manifest,
                              &config,
                              xs,
                              a.is_present("save"),
                              a.is_present("savedev"));
        result_exit("update", res);
    } else if let Some(a) = args.subcommand_matches("fetch") {
        let res = lal::fetch(&manifest, config, a.is_present("core"));
        result_exit("fetch", res);
    } else if let Some(a) = args.subcommand_matches("remove") {
        let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
        let res = lal::remove(manifest, xs, a.is_present("save"), a.is_present("savedev"));
        result_exit("remove", res);
    } else if let Some(a) = args.subcommand_matches("build") {
        let res = lal::build(&config,
                             &manifest,
                             a.value_of("component"),
                             a.value_of("configuration"),
                             a.is_present("release"),
                             a.value_of("with-version"),
                             a.is_present("strict"),
                             a.is_present("print"));
        result_exit("build", res);
    } else if let Some(_) = args.subcommand_matches("list-components") {
        result_exit("list-components", lal::build_list(&manifest))
    } else if let Some(a) = args.subcommand_matches("shell") {
        let xs = if a.is_present("cmd") {
            Some(a.values_of("cmd").unwrap().collect::<Vec<_>>())
        } else {
            None
        };
        result_exit("shell",
                    lal::shell(&config,
                               a.is_present("print"),
                               xs,
                               a.is_present("privileged")));
    } else if let Some(a) = args.subcommand_matches("script") {
        let xs = if a.is_present("parameters") {
            a.values_of("parameters").unwrap().collect::<Vec<_>>()
        } else {
            vec![]
        };
        result_exit("script",
                    lal::script(&config,
                                a.value_of("script").unwrap(),
                                xs,
                                a.is_present("privileged")));
    } else if let Some(_) = args.subcommand_matches("verify") {
        result_exit("verify", lal::verify(&manifest));
    } else if let Some(a) = args.subcommand_matches("status") {
        result_exit("status", lal::status(&manifest, a.is_present("full")));
    } else if let Some(a) = args.subcommand_matches("stash") {
        result_exit("stash",
                    lal::stash(&config, &manifest, a.value_of("name").unwrap()));
    } else if let Some(a) = args.subcommand_matches("export") {
        result_exit("export",
                    lal::export(&config,
                                a.value_of("component").unwrap(),
                                a.value_of("output")));
    }

    unreachable!("Subcommand valid, but not implemented");
}
