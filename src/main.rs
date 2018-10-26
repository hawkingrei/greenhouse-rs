#![feature(plugin)]
#![feature(type_ascription)]
#![plugin(rocket_codegen)]

#[macro_use]
pub mod compression;
pub mod file;
pub mod router;
pub mod util;

extern crate rand;

#[macro_use]
extern crate quick_error;
extern crate brotli;
extern crate clap;
extern crate flate2;
extern crate lz4;
extern crate rocket;
extern crate rocket_slog;
extern crate sloggers;
extern crate snap;
extern crate zstd;
#[macro_use(debug)]
extern crate slog;

use clap::{App, Arg};
use rocket::config::{Config, Environment};
use rocket_slog::{SlogFairing, SyncLogger};
use sloggers::{
    terminal::{Destination, TerminalLoggerBuilder},
    types::Severity,
    Build,
};

#[derive(Clone)]
pub struct CachePath(String);

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
