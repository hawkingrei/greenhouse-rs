#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate greenhouse;
extern crate rocket;

use clap::{App, Arg};
use rocket::config::{Config, Environment};
use rocket_slog::SlogFairing;
use sloggers::{
    terminal::{Destination, TerminalLoggerBuilder},
    types::Severity,
    Build,
};

use greenhouse::config::CachePath;
use greenhouse::router;

fn main() {
    let matches = App::new("greenhouse")
        .author("hawkingrei <hawkingrei@gmail.com>")
        .arg(
            Arg::with_name("dir")
                .long("dir")
                .takes_value(true)
                .required(true)
                .help("location to store cache entries on disk"),
        )
        .arg(
            Arg::with_name("host")
                .long("host")
                .takes_value(true)
                .help("host address to listen on"),
        )
        .arg(
            Arg::with_name("cache-port")
                .long("cache-port")
                .takes_value(true)
                .help("port to listen on for cache requests"),
        )
        .get_matches();
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let logger = builder.build().unwrap();

    let fairing = SlogFairing::new(logger);

    let _dir = matches.value_of("dir").unwrap().to_owned();
    let _host = matches.value_of("host").unwrap_or("0.0.0.0");

    let _cache_port = matches
        .value_of("cachePort")
        .unwrap_or("8888")
        .parse::<u16>()
        .unwrap();
    println!("port was passed in: {}", _cache_port);
    let config = Config::build(Environment::Staging)
        .address(_host)
        .port(_cache_port)
        .finalize()
        .unwrap();
    rocket::custom(config, false)
        .attach(fairing)
        .manage(CachePath(_dir.to_string()))
        .mount("/", routes![router::upload, router::get])
        .launch();
}
