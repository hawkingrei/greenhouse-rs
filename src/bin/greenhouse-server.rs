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
use std::path::Path;
use std::{thread, time};
use tokio::runtime::Runtime;

use greenhouse::config::CachePath;
use greenhouse::disk::get_disk_usage_prom;
use greenhouse::diskgc::lazy;
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
    let metrics_addr = format!("{}:{}", "0.0.0.0", _metrics_port);
    let mut rt = Runtime::new().unwrap();
    let metrics_dir = _dir.to_string().to_string();
    let gcpath = metrics_dir.clone();
    rt.spawn(lazy(move || {
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
        Ok(())
    }));
    rt.spawn(lazy(move || {
        println!("port was passed in: {}", metrics_addr);
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
    let ten_millis = time::Duration::from_millis(1000);
    rt.spawn(lazy(move || {
        loop {
            thread::sleep(ten_millis);
            get_disk_usage_prom(Path::new(&metrics_dir));
        }
        Ok(())
    }));
    let pathbuf = Path::new(&gcpath).to_path_buf();
    let gc = lazy::Lazygc::new(pathbuf.as_path(), 0.8);
    let gc_millis = time::Duration::from_millis(10000);
    rt.spawn(lazy(move || {
        loop {
            gc.clone().rocket();
            thread::sleep(gc_millis);
        }
        Ok(())
    }));
    rt.shutdown_on_idle().wait().unwrap();
}
