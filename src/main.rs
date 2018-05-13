#[macro_use]
extern crate slog;
//extern crate slog_async;
extern crate chrono;
extern crate slog_term;
#[macro_use]
extern crate clap;
extern crate config;
extern crate serde;
extern crate serde_json;
extern crate url;
#[macro_use]
extern crate serde_derive;

use clap::{App, Arg, ArgMatches, SubCommand};
use slog::Drain;
use std::error::Error;
use std::path::Path;
use std::process;

mod libhar;

const APP_NAME: &str = "Haricot";
const APP_ENV_PREFIX: &str = "HAR";

fn run(
    lgg: &slog::Logger,
    conf: &mut config::Config,
    matches: &ArgMatches,
) -> Result<(), Box<Error>> {
    let fnhar = matches.value_of("fnhar").unwrap();
    info!(lgg, "Reading HAR file"; "fnhar" => fnhar);
    let query_string_excludes: Vec<&str> =
        vec!["_", "countToFetch", "sortBy", "sortOrder", "startFrom"];
    let headers_excludes: Vec<&str> = vec![
        "Access-Control-Allow-Headers",
        "Access-Control-Allow-Origin",
        "Access-Control-Expose-Headers",
        "Date",
        "Server",
        "Transfer-Encoding",
        "X-Barco-notification-channel",
        "Origin",
        "Accept-Encoding",
        "Accept-Language",
        "Authorization",
        "Cache-Control",
        "Connection",
        "Cookie",
        "DNT",
        "Host",
        "Pragma",
        "Referer",
        "User-Agent",
        "X-Barco-resource",
        "X-Requested-With",
    ];

    let doc = libhar::read_file(fnhar)?;

    match matches.subcommand() {
        ("summary", Some(my_matches)) => {
            let short_url = !my_matches.is_present("with-query-string");
            if my_matches.is_present("ecs") {
                libhar::print_overview(
                    &doc,
                    short_url,
                    Some(&query_string_excludes),
                    Some(&headers_excludes),
                )?
            } else {
                libhar::print_overview(&doc, short_url, None, None)?
            }
        }
        ("entries", Some(my_matches)) => {
            println!("{}", doc.log.entries.len());
        }
        ("body", Some(my_matches)) => {
            let num = value_t!(my_matches, "num", usize)?;
            let which = my_matches.value_of("which").unwrap();
            let ecs = my_matches.is_present("ecs");
            libhar::print_body(&doc, num, &which, ecs);
        }
        _ => libhar::print_overview(&doc, false, None, None)?,
    }

    Ok(())
}

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new(APP_NAME)
        .version(crate_version!())
        .author(crate_authors!())
        .about("Parse HAR files")
        .arg(Arg::with_name("verbose")
             .short("v")
             .multiple(true)
             .help("Verbosity level (default=WARNING, 'v'=INFO, 'vv'=DEBUG, 'vvv'=TRACE)"))
        .arg(Arg::with_name("conf")
             .short("c")
             .value_name("FILE")
             .takes_value(true)
             .help("Path to config file. Format may be TOML, YAML, HJSON, JSON.")
        )
        .arg(Arg::with_name("fnhar")
             .short("f")
             .value_name("FILE")
             .required(true)
             .takes_value(true)
             .help("Path to HAR file.")
        )
        .subcommand(SubCommand::with_name("summary")
                    .about("Show a summary of the entries")
                    .arg(Arg::with_name("ecs")
                         .long("ecs")
                         .help("Exclude some query strings and headers suitable for analysis of ECS")
                    )
                    .arg(Arg::with_name("with-query-string")
                         .long("with-query-string")
                         .help("By default we print the URL without its query string (we list the query pairs anyways). If this flag is set, we print the URL as-is, including query string (if it has one).")
                    )
        )
        .subcommand(SubCommand::with_name("entries")
                    .about("Count entries")
        )
        .subcommand(SubCommand::with_name("body")
                    .about("Get body data")
                    .arg(Arg::with_name("ecs")
                         .long("ecs")
                         .help("Perform some transformations specific to ECS: (1) If body JSON contains field 'privateData', expand this into JSON as well.")
                    )
                    .arg(Arg::with_name("num")
                        .value_name("NUM")
                        .required(true)
                        .help("Get body of this entry")
                    )
                    .arg(Arg::with_name("which")
                        .value_name("WHICH")
                        .possible_values(&["req", "resp"])
                        .required(true)
                        .help("Get body of request 'req' or response 'resp'")
                    )
        )
        .get_matches()
}

fn build_config(matches: &ArgMatches) -> config::Config {
    let mut conf = config::Config::default();

    if let Some(c) = matches.value_of("conf") {
        let fn_conf = Path::new(c);
        conf.merge(config::File::from(fn_conf)).unwrap();
    }

    conf.merge(config::Environment::with_prefix(APP_ENV_PREFIX))
        .unwrap();

    conf.set("app_name", APP_NAME).unwrap();
    conf.set("app_version", crate_version!()).unwrap();
    conf.set("app_authors", crate_authors!()).unwrap();
    conf.set("min_log_level", matches.occurrences_of("verbose") as i64)
        .unwrap();
    conf
}

fn main() {
    let start_time = std::time::Instant::now();

    let matches = parse_args();
    let mut conf = build_config(&matches);

    // Configure logging
    let min_log_level = match conf.get_int("min_log_level").unwrap() {
        0 => slog::Level::Warning,
        1 => slog::Level::Info,
        2 => slog::Level::Debug,
        3 | _ => slog::Level::Trace,
    };
    let decorator = slog_term::PlainSyncDecorator::new(std::io::stderr());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    // FIXME If we use Async and App returns Error, we may not have log output!
    //let drain = slog_async::Async::new(drain).build().fuse();
    let drain = slog::LevelFilter::new(drain, min_log_level).fuse();
    let lgg_root = slog::Logger::root(drain, o!());

    // Run app
    info!(
        lgg_root,
        "Hello, {} Version {}",
        conf.get_str("app_name").unwrap(),
        conf.get_str("app_version").unwrap()
    );

    if let Err(e) = run(&lgg_root, &mut conf, &matches) {
        error!(lgg_root, "Application error"; "error" => format!("{:?}", e), "time_taken" => format!("{:?}", start_time.elapsed()));
        process::exit(1);
    }

    info!(lgg_root, "Finished."; "time_taken" => format!("{:?}", start_time.elapsed()));
}
