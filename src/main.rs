#![feature(plugin)]
#![feature(type_ascription)]
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
use rocket::fairing::AdHoc;
use rocket::http::uri::URI;
use rocket::request::State;
use rocket::Data;
use std::io;
use std::path::PathBuf;

struct Dir {
    dir: String,
}

#[put("/<file..>", data = "<paste>")]
fn upload(paste: Data, file: PathBuf, store: State<Dir>) -> io::Result<String> {
    let result = format!("{}/{}", store.dir, file.to_str().unwrap().to_string());
    return Ok(result);
}

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
        .manage(Dir { dir: _dir })
        .mount("/", routes![upload])
        .launch();
}
