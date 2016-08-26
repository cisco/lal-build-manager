#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;

extern crate lal;
use lal::{LalResult, Config, Manifest, StickyOptions};
use clap::{Arg, App, AppSettings, SubCommand};
use std::process;


fn is_integer(v: String) -> Result<(), String> {
    if v.parse::<u32>().is_ok() {
        return Ok(());
    }
    Err(format!("{} is not an integer", v))
}

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
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .global_settings(&[AppSettings::ColoredHelp])
        .about("lal dependency manager")
        .arg(Arg::with_name("environment")
            .short("e")
            .long("env")
            .takes_value(true)
            .help("Override the default environment for this command"))
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
            .arg(Arg::with_name("environment")
                .required(true)
                .help("Environment to build this component in"))
            .arg(Arg::with_name("force")
                .short("f")
                .help("overwrites manifest if necessary")))
        .subcommand(SubCommand::with_name("configure")
            .about("Creates a default lal config ~/.lal/"))
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
        .subcommand(SubCommand::with_name("env")
            .about("Manages environment configurations")
            .subcommand(SubCommand::with_name("set")
                .about("Override the default environment for this folder")
                .arg(Arg::with_name("environment")
                    .required(true)
                    .help("Name of the environment to use")))
            .subcommand(SubCommand::with_name("update").about("Update the current environment"))
            .subcommand(SubCommand::with_name("reset").about("Return to the default environment")))
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
                .help("Remove specific components")
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
        .subcommand(SubCommand::with_name("query")
            .about("Query for available versions on artifactory")
            .arg(Arg::with_name("component")
                .required(true)
                .help("Component name to search for")))
        .subcommand(SubCommand::with_name("update-all")
            .about("Update all dependencies in the manifest")
            .arg(Arg::with_name("dev")
                .short("D")
                .long("dev")
                .help("Update devDependencies instead of dependencies"))
            .arg(Arg::with_name("save")
                .short("S")
                .long("save")
                .help("Save updated versions in the right object in the manifest")))
        .subcommand(SubCommand::with_name("list-components")
            //.hidden(true) want this
            .about("list components that can be used with lal build"))
        .get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::init_with_verbosity(args.occurrences_of("verbose") + 1).unwrap();

    // Allow lal configure without assumptions
    if let Some(_) = args.subcommand_matches("configure") {
        result_exit("configure", lal::configure(true));
    }

    // Force config to exists before allowing remaining actions
    let config = Config::read()
        .map_err(|e| {
            error!("Configuration error: {}", e);
            println!("Ensure you have run `lal configure` and that ~/.lal/config is valid json");
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
        result_exit("init",
                    lal::init(&config,
                              a.is_present("force"),
                              a.value_of("environment").unwrap()));
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

    // Read .lalopts if it exists
    let stickies = StickyOptions::read()
        .map_err(|e| {
            // Should not happen unless people are mucking with it manually
            error!("Options error: {}", e);
            println!(".lalopts must be valid json");
            process::exit(1);
        })
        .unwrap(); // we get a default empty options here otherwise

    // Force a valid container key configured in manifest and corr. value in config
    // NB: --env overrides sticky env overrides manifest.env overrides centos
    let env = if args.is_present("environment") {
        args.value_of("environment").unwrap().into()
    } else if stickies.env.is_some() {
        stickies.env.clone().unwrap()
    } else if let Some(ref menv) = manifest.environment {
        menv.clone()
    } else {
        // temporary arm while manifest.environment is not mandatory
        "centos".into()
    };

    // lookup associated container
    let container = config.get_container(Some(env.clone()))
        .map_err(|e| {
            error!("Environment error: {}", e);
            println!("Ensure that manifest.environment has a corresponding entry in ~/.lal/config");
            process::exit(1);
        })
        .unwrap();

    // resolve env updates and sticky options before main subcommands
    if let Some(a) = args.subcommand_matches("env") {
        if let Some(_) = a.subcommand_matches("update") {
            result_exit("env update", lal::env::update(&container, &env))
        } else if let Some(_) = a.subcommand_matches("reset") {
            // NB: if .lalopts.env points at an environment not in config
            // reset will fail.. possible to fix, but complects this file too much
            // .lalopts writes are checked in lal::env::set anyway so this
            // would be purely the users fault for editing it manually
            result_exit("env clear", lal::env::clear())
        } else if let Some(sa) = a.subcommand_matches("set") {
            result_exit("env override",
                        lal::env::set(&stickies, &config, sa.value_of("environment").unwrap()))
        } else {
            // just print current environment
            println!("{}", env);
            process::exit(0);
        }
    }

    // Remaining actions - assume Manifest, Config, and Container

    // Subcommands that are environment agnostic
    if let Some(a) = args.subcommand_matches("status") {
        result_exit("status", lal::status(&manifest, a.is_present("full")));
    } else if let Some(_) = args.subcommand_matches("list-components") {
        result_exit("list-components", lal::build_list(&manifest))
    } else if let Some(a) = args.subcommand_matches("remove") {
        let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
        let res = lal::remove(&manifest, xs, a.is_present("save"), a.is_present("savedev"));
        result_exit("remove", res);
    } else if let Some(a) = args.subcommand_matches("stash") {
        result_exit("stash",
                    lal::stash(&config, &manifest, a.value_of("name").unwrap()));
    }

    // Warn users who are overriding the default for the main commands
    if manifest.environment.is_some() && manifest.environment.clone().unwrap() != env {
        let sub = args.subcommand_name().unwrap();
        warn!("Running {} command for {} environment", sub, env);
    }

    // Main subcommands
    if let Some(a) = args.subcommand_matches("update") {
        let xs = a.values_of("components").unwrap().map(|s| s.to_string()).collect::<Vec<_>>();
        let res = lal::update(manifest,
                              &config,
                              xs,
                              a.is_present("save"),
                              a.is_present("savedev"));
        result_exit("update", res);
    } else if let Some(a) = args.subcommand_matches("update-all") {
        let res = lal::update_all(manifest, &config, a.is_present("save"), a.is_present("dev"));
        result_exit("update-all", res);
    } else if let Some(a) = args.subcommand_matches("fetch") {
        let res = lal::fetch(&manifest, config, a.is_present("core"));
        result_exit("fetch", res);
    } else if let Some(a) = args.subcommand_matches("build") {
        let res = lal::build(&config,
                             &manifest,
                             a.value_of("component"),
                             a.value_of("configuration"),
                             a.is_present("release"),
                             a.value_of("with-version"),
                             a.is_present("strict"),
                             &container,
                             env,
                             a.is_present("print"));
        result_exit("build", res);
    } else if let Some(a) = args.subcommand_matches("shell") {
        let xs = if a.is_present("cmd") {
            Some(a.values_of("cmd").unwrap().collect::<Vec<_>>())
        } else {
            None
        };
        result_exit("shell",
                    lal::shell(&config,
                               &container,
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
                                &container,
                                a.value_of("script").unwrap(),
                                xs,
                                a.is_present("privileged")));
    } else if let Some(_) = args.subcommand_matches("verify") {
        result_exit("verify", lal::verify(&manifest, env));
    } else if let Some(a) = args.subcommand_matches("query") {
        result_exit("query",
                    lal::query(&config, a.value_of("component").unwrap()));
    } else if let Some(a) = args.subcommand_matches("export") {
        result_exit("export",
                    lal::export(&config,
                                a.value_of("component").unwrap(),
                                a.value_of("output")));
    }

    unreachable!("Subcommand valid, but not implemented");
}
