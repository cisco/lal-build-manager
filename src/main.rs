#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate openssl_probe;

extern crate lal;
use lal::*;
use clap::{Arg, App, AppSettings, SubCommand, ArgMatches};
use std::process;
use std::ops::Deref;

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

// functions that work without a manifest, and thus can run without a set env
fn handle_manifest_agnostic_cmds(
    args: &ArgMatches,
    cfg: &Config,
    backend: &Backend,
    explicit_env: Option<&str>,
) {
    let res = if let Some(a) = args.subcommand_matches("export") {
        lal::export(backend,
                    a.value_of("component").unwrap(),
                    a.value_of("output"),
                    explicit_env)
    } else if let Some(a) = args.subcommand_matches("query") {
        lal::query(backend,
                   explicit_env,
                   a.value_of("component").unwrap(),
                   a.is_present("latest"))
    } else if let Some(a) = args.subcommand_matches("publish") {
        lal::publish(a.value_of("component").unwrap(), backend)
    } else if args.subcommand_matches("list-environments").is_some() {
        lal::list::environments(cfg)
    } else {
        return ();
    };
    result_exit(args.subcommand_name().unwrap(), res);
}

// functions that need a manifest, but do not depend on environment values
fn handle_environment_agnostic_cmds(args: &ArgMatches, mf: &Manifest, backend: &Backend) {
    let res = if let Some(a) = args.subcommand_matches("status") {
        lal::status(mf,
                    a.is_present("full"),
                    a.is_present("origin"),
                    a.is_present("time"))
    } else if args.subcommand_matches("list-components").is_some() {
        lal::list::buildables(mf)
    } else if args.subcommand_matches("list-supported-environments").is_some() {
        lal::list::supported_environments(mf)
    } else if let Some(a) = args.subcommand_matches("list-configurations") {
        lal::list::configurations(a.value_of("component").unwrap(), mf)
    } else if let Some(a) = args.subcommand_matches("list-dependencies") {
        lal::list::dependencies(mf, a.is_present("core"))
    } else if let Some(a) = args.subcommand_matches("remove") {
        let xs = a.values_of("components").unwrap().map(String::from).collect::<Vec<_>>();
        lal::remove(mf, xs, a.is_present("save"), a.is_present("savedev"))
    } else if let Some(a) = args.subcommand_matches("stash") {
        lal::stash(backend, mf, a.value_of("name").unwrap())
    } else if let Some(a) = args.subcommand_matches("propagate") {
        lal::propagate::print(mf, a.value_of("component").unwrap(), a.is_present("json"))
    } else {
        return ();
    };
    result_exit(args.subcommand_name().unwrap(), res);
}

fn handle_network_cmds(args: &ArgMatches, mf: &Manifest, backend: &Backend, env: &str) {
    let res = if let Some(a) = args.subcommand_matches("update") {
        let xs = a.values_of("components").unwrap().map(String::from).collect::<Vec<_>>();
        lal::update(mf,
                    backend,
                    xs,
                    a.is_present("save"),
                    a.is_present("savedev"),
                    env)
    } else if let Some(a) = args.subcommand_matches("update-all") {
        lal::update_all(mf, backend, a.is_present("save"), a.is_present("dev"), env)
    } else if let Some(a) = args.subcommand_matches("fetch") {
        lal::fetch(mf, backend, a.is_present("core"), env)
    } else {
        return (); // not a network cmnd
    };
    result_exit(args.subcommand_name().unwrap(), res)
}

fn handle_env_command(
    args: &ArgMatches,
    cfg: &Config,
    env: &str,
    stickies: &StickyOptions,
) -> Container {

    // lookup associated container from
    let container = cfg.get_container(env.into())
        .map_err(|e| {
            error!("Environment error: {}", e);
            println!("Ensure that manifest.environment has a corresponding entry in ~/.lal/config");
            process::exit(1);
        })
        .unwrap();

    // resolve env updates and sticky options before main subcommands
    if let Some(a) = args.subcommand_matches("env") {
        if a.subcommand_matches("update").is_some() {
            result_exit("env update", lal::env::update(&container, env))
        } else if a.subcommand_matches("reset").is_some() {
            // NB: if .lal/opts.env points at an environment not in config
            // reset will fail.. possible to fix, but complects this file too much
            // .lal/opts writes are checked in lal::env::set anyway so this
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

#[cfg(feature = "upgrade")]
fn handle_upgrade(args: &ArgMatches, cfg: &Config) {
    // we have a subcommand because SubcommandRequiredElseHelp
    let subname = args.subcommand_name().unwrap();

    // Allow lal upgrade without manifest
    if args.subcommand_matches("upgrade").is_some() {
        result_exit("upgrade", lal::upgrade(false)); // explicit, verbose check
    }

    // Autoupgrade if enabled - runs once daily if enabled
    // also excluding all listers because they are used in autocomplete
    if cfg.autoupgrade && subname != "upgrade" && !subname.contains("list-") &&
        cfg.upgrade_check_time()
    {
        debug!("Performing daily upgrade check");
        let _ = lal::upgrade(false).map_err(|e| {
            error!("Daily upgrade check failed: {}", e);
            // don't halt here if this ever happens as it could break it for users
        });
        let _ = cfg.clone().performed_upgrade().map_err(|e| {
            error!("Daily upgrade check updating lastUpgrade failed: {}", e);
            // Ditto
        });
        debug!("Upgrade check done - continuing to requested operation\n");
    }
}



fn handle_docker_cmds(
    args: &ArgMatches,
    mf: &Manifest,
    cfg: &Config,
    env: &str,
    container: &Container,
) {
    let res = if let Some(a) = args.subcommand_matches("verify") {
        // not really a docker related command, but it needs
        // the resolved env to verify consistent dependency usage
        lal::verify(mf, env, a.is_present("simple"))
    } else if let Some(a) = args.subcommand_matches("build") {
        let bopts = BuildOptions {
            name: a.value_of("component").map(String::from),
            configuration: a.value_of("configuration").map(String::from),
            release: a.is_present("release"),
            version: a.value_of("with-version").map(String::from),
            sha: a.value_of("with-sha").map(String::from),
            container: container.clone(),
            force: a.is_present("force"),
            simple_verify: a.is_present("simple-verify"),
        };
        let modes = ShellModes {
            printonly: a.is_present("print"),
            x11_forwarding: a.is_present("x11"),
            host_networking: a.is_present("net-host"),
            env_vars: values_t!(a.values_of("env-var"), String).unwrap_or(vec![]),
        };
        lal::build(cfg, mf, &bopts, env.into(), modes)
    } else if let Some(a) = args.subcommand_matches("shell") {
        let xs = if a.is_present("cmd") {
            Some(a.values_of("cmd").unwrap().collect::<Vec<_>>())
        } else {
            None
        };
        let modes = ShellModes {
            printonly: a.is_present("print"),
            x11_forwarding: a.is_present("x11"),
            host_networking: a.is_present("net-host"),
            env_vars: values_t!(a.values_of("env-var"), String).unwrap_or(vec![]),
        };
        lal::shell(cfg, container, &modes, xs, a.is_present("privileged"))
    } else if let Some(a) = args.subcommand_matches("run") {
        let xs = if a.is_present("parameters") {
            a.values_of("parameters").unwrap().collect::<Vec<_>>()
        } else {
            vec![]
        };
        let modes = ShellModes {
            printonly: a.is_present("print"),
            x11_forwarding: a.is_present("x11"),
            host_networking: a.is_present("net-host"),
            env_vars: values_t!(a.values_of("env-var"), String).unwrap_or(vec![]),
        };
        lal::script(cfg,
                    container,
                    a.value_of("script").unwrap(),
                    xs,
                    &modes,
                    a.is_present("privileged"))
    } else {
        return (); // no valid docker related command found
    };
    result_exit(args.subcommand_name().unwrap(), res);
}

fn main() {
    let mut app = App::new("lal")
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
            .help("Increase verbosity"))
       .arg(Arg::with_name("debug")
            .short("d")
            .long("debug")
            .help("Adds line numbers to log statements"))
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
            .arg(Arg::with_name("simple-verify")
                .short("s")
                .long("simple-verify")
                .help("Use verify --simple to check INPUT (allows stashed dependencies)"))
            .arg(Arg::with_name("force")
                .long("force")
                .short("f")
                .help("Ignore verify errors when using custom dependencies"))
            .arg(Arg::with_name("release")
                .long("release")
                .short("r")
                .help("Create a release tarball that can be published"))
            .arg(Arg::with_name("with-version")
                .long("with-version")
                .takes_value(true)
                .requires("release")
                .help("Configure lockfiles with an explicit version number"))
            .arg(Arg::with_name("with-sha")
                .long("with-sha")
                .takes_value(true)
                .requires("release")
                .help("Configure lockfiles with an explicit sha"))
            .arg(Arg::with_name("x11")
                .short("X")
                .long("X11")
                .help("Enable best effort X11 forwarding"))
            .arg(Arg::with_name("net-host")
                .short("n")
                .long("net-host")
                .help("Enable host networking"))
            .arg(Arg::with_name("env-var")
                .long("env-var")
                .help("Set environment variables in the container")
                .multiple(true)
                .takes_value(true)
                .number_of_values(1))
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
        .subcommand(SubCommand::with_name("verify")
            .arg(Arg::with_name("simple")
                .short("s")
                .long("simple")
                .help("Allow stashed versions in this simpler verify algorithm"))
            .about("verify consistency of INPUT"))
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
            .arg(Arg::with_name("x11")
                .short("X")
                .long("X11")
                .help("Enable X11 forwarding (best effort)"))
            .arg(Arg::with_name("net-host")
                .short("n")
                .long("net-host")
                .help("Enable host networking"))
            .arg(Arg::with_name("env-var")
                .long("env-var")
                .help("Set environment variables in the container")
                .multiple(true)
                .takes_value(true)
                .number_of_values(1))
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
            .arg(Arg::with_name("x11")
                .short("X")
                .long("X11")
                .help("Enable X11 forwarding (best effort)"))
            .arg(Arg::with_name("net-host")
                .short("n")
                .long("net-host")
                .help("Enable host networking"))
            .arg(Arg::with_name("env-var")
                .long("env-var")
                .help("Set environment variables in the container")
                .multiple(true)
                .takes_value(true)
                .number_of_values(1))
            .arg(Arg::with_name("print")
                .long("print-only")
                .help("Only print the docker run command and exit"))
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
            .about("Creates a default lal config ~/.lal/ from a defaults file")
            .arg(Arg::with_name("file")
                .required(true)
                .help("An environments file to seed the config with")))
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
            .arg(Arg::with_name("latest")
                .long("latest")
                .short("l")
                .help("Return latest version only"))
            .arg(Arg::with_name("component")
                .required(true)
                .help("Component name to search for")))
        .subcommand(SubCommand::with_name("propagate")
            .about("Show steps to propagate a version fully through the tree")
            .arg(Arg::with_name("component")
                .required(true)
                .help("Component to propagate"))
            .arg(Arg::with_name("json")
                .short("j")
                .long("json")
                .help("Produce a machine readable instruction set")))
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
        .subcommand(SubCommand::with_name("list-supported-environments")
            .setting(AppSettings::Hidden)
            .about("list supported environments from the manifest"))
        .subcommand(SubCommand::with_name("list-environments")
            .setting(AppSettings::Hidden)
            .about("list environments that can be used with lal build"))
        .subcommand(SubCommand::with_name("list-configurations")
            .setting(AppSettings::Hidden)
            .arg(Arg::with_name("component")
                .required(true)
                .help("Component name to look for in the manifest"))
            .about("list configurations for a given component"))
        .subcommand(SubCommand::with_name("list-dependencies")
            .setting(AppSettings::Hidden)
            .arg(Arg::with_name("core")
                .short("c")
                .long("core")
                .help("Only list core dependencies"))
            .about("list dependencies from the manifest"));

    if cfg!(feature = "upgrade") {
        app = app.subcommand(SubCommand::with_name("upgrade")
                                 .about("Attempts to upgrade lal from artifactory"));
    }

    let args = app.get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::Logger::new()
        .verbosity(args.occurrences_of("verbose") + 1)
        .module_path(true)
        .line_numbers(args.is_present("debug"))
        .init()
        .unwrap();

    // Allow lal configure without assumptions
    if let Some(a) = args.subcommand_matches("configure") {
        result_exit("configure",
                    lal::configure(true, true, a.value_of("file").unwrap()));
    }

    // Force config to exists before allowing remaining actions
    let config = Config::read()
        .map_err(|e| {
            error!("Configuration error: {}", e);
            println!("");
            println!("If you just got upgraded use `lal configure <site-config>`");
            println!("Site configs are found in {{install_prefix}}/share/lal/configs/ \
                      and should auto-complete");
            process::exit(1);
        })
        .unwrap();

    // Create a storage backend (something that implements storage/traits.rs)
    let backend: Box<Backend> = match &config.backend {
        &BackendConfiguration::Artifactory(ref art_cfg) => {
            Box::new(ArtifactoryBackend::new(&art_cfg, &config.cache))
        }
        &BackendConfiguration::Local(ref local_cfg) => {
            Box::new(LocalBackend::new(&local_cfg, &config.cache))
        }
    };

    // Ensure SSL is initialized before using the backend
    openssl_probe::init_ssl_cert_env_vars();

    // Do upgrade checks or handle explicit `lal upgrade` here
    #[cfg(feature = "upgrade")] handle_upgrade(&args, &config);

    // Allow lal init / clean without manifest existing in PWD
    if let Some(a) = args.subcommand_matches("init") {
        result_exit("init",
                    lal::init(&config,
                              a.is_present("force"),
                              a.value_of("environment").unwrap()));
    } else if let Some(a) = args.subcommand_matches("clean") {
        let days = a.value_of("days").unwrap().parse().unwrap();
        result_exit("clean", lal::clean(&config.cache, days));
    }

    // Read .lal/opts if it exists
    let stickies = StickyOptions::read()
        .map_err(|e| {
            // Should not happen unless people are mucking with it manually
            error!("Options error: {}", e);
            println!(".lal/opts must be valid json");
            process::exit(1);
        })
        .unwrap(); // we get a default empty options here otherwise

    // Manifest agnostic commands need explicit environments to not look in global location
    let explicit_env = args.value_of("environment");
    if let Some(env) = explicit_env {
        config
            .get_container(env.into())
            .map_err(|e| {
                error!("Environment error: {}", e);
                process::exit(1)
            })
            .unwrap();
    }
    handle_manifest_agnostic_cmds(&args, &config, backend.deref(), explicit_env);

    // Force manifest to exist before allowing remaining actions
    let manifest = Manifest::read()
        .map_err(|e| {
            error!("Manifest error: {}", e);
            println!("Ensure manifest.json is valid json or run `lal init`");
            process::exit(1);
        })
        .unwrap();

    // Subcommands that are environment agnostic
    handle_environment_agnostic_cmds(&args, &manifest, backend.deref());

    // Force a valid container key configured in manifest and corr. value in config
    // NB: --env overrides sticky env overrides manifest.env
    let env = if let Some(eflag) = args.value_of("environment") {
        eflag.into()
    } else if let Some(ref stickenv) = stickies.env {
        stickenv.clone()
    } else {
        manifest.environment.clone()
    };
    let container = handle_env_command(&args, &config, &env, &stickies);

    // Warn users who are using an unsupported environment
    if !manifest.supportedEnvironments.clone().into_iter().any(|e| e == env) {
        let sub = args.subcommand_name().unwrap();
        warn!("Running {} command in unsupported {} environment", sub, env);
    } else {
        let sub = args.subcommand_name().unwrap();
        debug!("Running {} command in supported {} environent", sub, env);
    }

    // Main subcommands
    handle_network_cmds(&args, &manifest, backend.deref(), &env);
    handle_docker_cmds(&args, &manifest, &config, &env, &container);

    unreachable!("Subcommand valid, but not implemented");
}
