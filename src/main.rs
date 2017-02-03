#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;

extern crate lal;
use lal::{LalResult, Config, Manifest, StickyOptions, Container};
use clap::{Arg, App, AppSettings, SubCommand, ArgMatches};
use std::process;
use std::env;

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
        debug!("{}: {:?}", name, e); // in the off-chance that Debug is useful
        process::exit(1);
    });
    process::exit(0);
}

// functions that work without a manifest, and thus with only partian env override ability
fn handle_manifest_agnostic_cmds(args: &ArgMatches, cfg: &Config, env_partial: &str) {
    let res = if let Some(a) = args.subcommand_matches("export") {
        lal::export(cfg,
                    a.value_of("component").unwrap(),
                    a.value_of("output"),
                    env_partial)
    } else if let Some(a) = args.subcommand_matches("query") {
        lal::query(cfg, a.value_of("component").unwrap())
    } else {
        return ();
    };
    result_exit(args.subcommand_name().unwrap(), res);
}

fn handle_environment_agnostic_cmds(args: &ArgMatches, mf: &Manifest, cfg: &Config) {
    let res = if let Some(a) = args.subcommand_matches("status") {
        lal::status(mf,
                    a.is_present("full"),
                    a.is_present("origin"),
                    a.is_present("time"))
    } else if let Some(_) = args.subcommand_matches("list-components") {
        lal::build_list(mf)
    } else if let Some(_) = args.subcommand_matches("list-environments") {
        lal::env_list(cfg)
    } else if let Some(a) = args.subcommand_matches("list-dependencies") {
        lal::dep_list(mf, a.is_present("core"))
    } else if let Some(a) = args.subcommand_matches("remove") {
        let xs = a.values_of("components").unwrap().collect::<Vec<_>>();
        lal::remove(mf, xs, a.is_present("save"), a.is_present("savedev"))
    } else if let Some(a) = args.subcommand_matches("stash") {
        lal::stash(cfg, mf, a.value_of("name").unwrap())
    } else {
        return ();
    };
    result_exit(args.subcommand_name().unwrap(), res);
}

fn handle_network_cmds(args: &ArgMatches, mf: &Manifest, cfg: &Config, env: &str) {
    let res = if let Some(a) = args.subcommand_matches("update") {
        let xs = a.values_of("components").unwrap().map(|s| s.to_string()).collect::<Vec<_>>();
        lal::update(mf,
                    cfg,
                    xs,
                    a.is_present("save"),
                    a.is_present("savedev"),
                    env)
    } else if let Some(a) = args.subcommand_matches("update-all") {
        lal::update_all(mf, cfg, a.is_present("save"), a.is_present("dev"), env)
    } else if let Some(a) = args.subcommand_matches("fetch") {
        lal::fetch(mf, cfg, a.is_present("core"), env)
    } else {
        return (); // not a network cmnd
    };
    result_exit(args.subcommand_name().unwrap(), res)
}

fn handle_env_command(args: &ArgMatches,
                      cfg: &Config,
                      env_: &str,
                      stickies: &StickyOptions)
                      -> Container {
    let env = if env_ == "default" { "centos".to_string() } else { env_.to_string() };

    // lookup associated container from
    let container = cfg.get_container(Some(env.clone()))
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
                        lal::env::set(stickies, cfg, sa.value_of("environment").unwrap()))
        } else {
            // just print current environment
            println!("{}", env);
            process::exit(0);
        }
    }
    // if we didn't handle an env subcommand here return the container
    // needs to be resolved later on for docker cmds anyway
    container
}

fn handle_docker_cmds(args: &ArgMatches,
                      mf: &Manifest,
                      cfg: &Config,
                      env_: &str,
                      container: &Container) {
    let env = if env_ == "default" { "centos".to_string() } else { env_.to_string() };
    let res = if let Some(_) = args.subcommand_matches("verify") {
        // not really a docker related command, but it needs
        // the resolved env to verify consistent dependency usage
        lal::verify(mf, &env)
    } else if let Some(a) = args.subcommand_matches("publish") {
        // ditto for publish, because it needs verify
        lal::publish(a.value_of("component").unwrap(), cfg, &env)
    } else if let Some(a) = args.subcommand_matches("build") {
        if a.is_present("strict") {
            warn!("deprecation: build --strict is now default behaviour");
        }
        lal::build(cfg,
                   mf,
                   a.value_of("component"),
                   a.value_of("configuration"),
                   a.is_present("release"),
                   a.value_of("with-version"),
                   !a.is_present("force"),
                   container,
                   env,
                   a.is_present("print"))
    } else if let Some(a) = args.subcommand_matches("shell") {
        let xs = if a.is_present("cmd") {
            Some(a.values_of("cmd").unwrap().collect::<Vec<_>>())
        } else {
            None
        };
        lal::shell(cfg,
                   container,
                   a.is_present("print"),
                   xs,
                   a.is_present("privileged"))
    } else if let Some(a) = args.subcommand_matches("run") {
        let xs = if a.is_present("parameters") {
            a.values_of("parameters").unwrap().collect::<Vec<_>>()
        } else {
            vec![]
        };
        lal::script(cfg,
                    container,
                    a.value_of("script").unwrap(),
                    xs,
                    a.is_present("privileged"))
    } else {
        return (); // no valid docker related command found
    };
    result_exit(args.subcommand_name().unwrap(), res);
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
            .arg(Arg::with_name("force")
                .long("force")
                .short("f")
                .help("Ignore verify error when using custom dependencies"))
            .arg(Arg::with_name("strict")
                .long("strict")
                .short("s")
                .help("DEPRECATED - block on verify (default behaviour)"))
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
            .arg(Arg::with_name("time")
                .short("t")
                .long("time")
                .help("Print build time of artifact"))
            .arg(Arg::with_name("origin")
                .short("o")
                .long("origin")
                .help("Print version and environment origin of artifact"))
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
        .subcommand(SubCommand::with_name("run")
            .about("Runs scripts from .lal/scripts in the configured container")
            .alias("script")
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
        .subcommand(SubCommand::with_name("publish")
            .setting(AppSettings::Hidden)
            .arg(Arg::with_name("component")
                .required(true)
                .help("Component name to publish"))
            .about("Publish a release build to the default artifactory location"))
        .subcommand(SubCommand::with_name("list-components")
            .setting(AppSettings::Hidden)
            .about("list components that can be used with lal build"))
        .subcommand(SubCommand::with_name("list-environments")
            .setting(AppSettings::Hidden)
            .about("list environments that can be used with lal build"))
        .subcommand(SubCommand::with_name("list-dependencies")
            .setting(AppSettings::Hidden)
            .arg(Arg::with_name("core")
                .short("c")
                .long("core")
                .help("Only list core dependencies"))
            .about("list dependencies from the manifest"))
        .get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::init_with_verbosity(args.occurrences_of("verbose") + 1).unwrap();

    // set ssl cert path early for hyper client
    match env::var_os("SSL_CERT_FILE") {
        Some(val) => trace!("Using SSL_CERT_FILE set to {:?}", val),
        // By default point it to normal location (wont work for centos)
        None =>  env::set_var("SSL_CERT_FILE", "/etc/ssl/certs/ca-certificates.crt")
    }

    // we have a subcommand because SubcommandRequiredElseHelp
    let subname = args.subcommand_name().unwrap();

    // Allow lal configure without assumptions
    if let Some(_) = args.subcommand_matches("configure") {
        result_exit("configure", lal::configure(true, true));
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
    // also excluding all listers because they are used in autocomplete
    if subname != "upgrade" && !subname.contains("list-") && config.upgrade_check_time() {
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

    // Read .lalopts if it exists
    let stickies = StickyOptions::read()
        .map_err(|e| {
            // Should not happen unless people are mucking with it manually
            error!("Options error: {}", e);
            println!(".lalopts must be valid json");
            process::exit(1);
        })
        .unwrap(); // we get a default empty options here otherwise

    // Work out a partial env for manifest agnostic commands:
    let env_early = if let Some(eflag) = args.value_of("environment") {
        eflag.into()
    } else if let Some(ref stickenv) = stickies.env {
        stickenv.clone()
    } else {
        "default".into()
    };
    handle_manifest_agnostic_cmds(&args, &config, &env_early);

    // Force manifest to exist before allowing remaining actions
    let manifest = Manifest::read()
        .map_err(|e| {
            error!("Manifest error: {}", e);
            println!("Ensure manifest.json is valid json or run `lal init`");
            process::exit(1);
        })
        .unwrap();

    // Subcommands that are environment agnostic
    handle_environment_agnostic_cmds(&args, &manifest, &config);

    // Force a valid container key configured in manifest and corr. value in config
    // NB: --env overrides sticky env overrides manifest.env overrides centos
    let env = if let Some(eflag) = args.value_of("environment") {
        eflag.into()
    } else if let Some(ref stickenv) = stickies.env {
        stickenv.clone()
    } else if let Some(ref menv) = manifest.environment {
        menv.clone()
    } else {
        // nothing specified - defaults inferred differently later on:
        // - docker commands translate this to "centos"
        // - network ones translate it to the default artifactory location
        "default".into()
    };
    let container = handle_env_command(&args, &config, &env, &stickies);

    // Warn users who are overriding the default for the main commands
    if let Some(ref menv) = manifest.environment {
        if *menv != env {
            let sub = args.subcommand_name().unwrap();
            warn!("Running {} command in non-default {} environment", sub, env);
        }
    } else {
        warn!("Manifest is missing an environment value");
        warn!("Please hardcode an environment inside manifest.json with a value from \
               ~/.lal/config");
    }

    // Main subcommands
    handle_network_cmds(&args, &manifest, &config, &env);
    handle_docker_cmds(&args, &manifest, &config, &env, &container);

    unreachable!("Subcommand valid, but not implemented");
}
