#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
pub mod compression;
pub mod util;

extern crate rand;

#[macro_use]
extern crate quick_error;
extern crate brotli;
extern crate clap;
extern crate flate2;
extern crate lz4;
extern crate rocket;
extern crate snap;
extern crate zstd;

use clap::{App, Arg};
use rocket::config::{Config, Environment};
use rocket::http::uri::URI;
use rocket::Data;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[put("/", data = "<paste>")]
fn upload(paste: Data, uri: &URI) -> &'static str {
    "Hello, world!"
}

fn main() {
    let matches = App::new("greenhouse")
        .author("hawkingrei <hawkingrei@gmail.com>")
        .arg(
            Arg::with_name("dir")
                .takes_value(true)
                .value_name("DIR")
                .help("location to store cache entries on disk"),
        )
        .arg(
            Arg::with_name("host")
                .takes_value(true)
                .value_name("HOST")
                .help("host address to listen on")
        )
        .arg(Arg::with_name("cache-port").takes_value(true).value_name("cachePort").help("port to listen on for cache requests"))
        .arg(Arg::with_name("metrics-port").takes_value(true).value_name("metricsPort").help("port to listen on for prometheus metrics scraping"))
        .arg(Arg::with_name("metrics-update-interval").takes_value(true).value_name("metricsUpdateInterval").help("interval between updating disk metrics"))
        .arg(
            Arg::with_name("min-percent-blocks-free")
                .takes_value(true)
                .value_name("minPercentBlocksFree")
                .help("minimum percent of blocks free on --dir's disk before evicting entries"),
        )
        .arg(
            Arg::with_name("evict-until-percent-blocks-free")
                .takes_value(true)
                .value_name("evictUntilPercentBlocksFree")
                .help("continue evicting from the cache until at least this percent of blocks are free"),
        )
        .arg(
            Arg::with_name("disk-check-interval")
                .takes_value(true)
                .value_name("diskCheckInterval")
                .help("interval between checking disk usage (and potentially evicting entries"),
        )
        .get_matches();
    let _dir = matches.value_of("DIR").unwrap();
    let _host = matches.value_of("HOST").unwrap_or("0.0.0.0");
    let _cache_port = matches
        .value_of("cachePort")
        .unwrap_or("8888")
        .parse::<u16>()
        .unwrap();
    let config = Config::build(Environment::Staging)
        .address(_host)
        .port(_cache_port)
        .finalize()
        .unwrap();
    rocket::custom(config, false)
        .mount("/", routes![index, upload])
        .launch();
}
