#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;

extern crate lal;
use lal::{LalResult, Config, Manifest};

use std::process;

// cli.rs included raw, as it is also used in the build.rs script for bash completions
// this exposes the `application` function with the clap App instance in here
include!("cli.rs");

fn result_exit<T>(name: &str, x: LalResult<T>) {
    let _ = x.map_err(|e| {
        println!(""); // add a separator
        error!("{} error: {}", name, e);
        process::exit(1);
    });
    process::exit(0);
}

fn main() {
    let args = application().get_matches();

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
        let xs = a.values_of("components").unwrap().map(|s| s.to_string()).collect::<Vec<_>>();
        let res = lal::update(manifest,
                              &config,
                              xs,
                              a.is_present("save"),
                              a.is_present("savedev"));
        result_exit("update", res);
    } else if let Some(a) = args.subcommand_matches("update-all") {
        let res = lal::update_all(manifest,
                                  &config,
                                  a.is_present("save"),
                                  a.is_present("dev"));
        result_exit("update-all", res);
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
    } else if let Some(a) = args.subcommand_matches("query") {
        result_exit("query", lal::query(&config, a.value_of("component").unwrap()));
    } else if let Some(a) = args.subcommand_matches("export") {
        result_exit("export",
                    lal::export(&config,
                                a.value_of("component").unwrap(),
                                a.value_of("output")));
    }

    unreachable!("Subcommand valid, but not implemented");
}
