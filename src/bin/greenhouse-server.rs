#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate greenhouse;
extern crate hyper;
extern crate rocket;

use clap::{App, Arg};
use futures::future::lazy;
use futures::Future;
use rocket::config::{Config, Environment};
use rocket_slog::SlogFairing;
use sloggers::{
    terminal::{Destination, TerminalLoggerBuilder},
    types::Severity,
    Build,
};
use tokio::runtime::Runtime;

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
        .arg(
            Arg::with_name("metrics-port")
                .long("metrics-port")
                .takes_value(true)
                .help("port to listen on for prometheus metrics scraping"),
        )
        .get_matches();
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let logger = builder.build().unwrap();

    let fairing = SlogFairing::new(logger);

    let _dir = matches.value_of("dir").unwrap_or("~/tmp/cache").to_owned();
    let _host = matches.value_of("host").unwrap_or("0.0.0.0").to_owned();;

    let _cache_port = matches
        .value_of("cachePort")
        .unwrap_or("8080")
        .parse::<u16>()
        .unwrap();
    let _metrics_port = matches
        .value_of("metrics-port")
        .unwrap_or("9090")
        .parse::<u16>()
        .unwrap();
    println!("port was passed in: {}", _cache_port);
    let metrics_addr = format!("{}:{}", "0.0.0.0", _metrics_port);
    println!("port was passed in: {}", metrics_addr);
    let mut rt = Runtime::new().unwrap();
    rt.spawn(lazy(move || {
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
        Ok(())
    }));
    rt.spawn(lazy(move || {
        let config = Config::build(Environment::Staging)
            .address("0.0.0.0")
            .port(_metrics_port)
            .finalize()
            .unwrap();
        rocket::custom(config, false)
            .mount("/", routes![router::metrics_router::metrics])
            .launch();
        Ok(())
    }));
    rt.shutdown_on_idle().wait().unwrap();
}
