#![feature(plugin)]
#![feature(type_ascription)]
#![plugin(rocket_codegen)]

#[macro_use]
pub mod compression;
pub mod file;
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
#[macro_use(info)]
extern crate slog;

use self::file::CacheFile;
use clap::{App, Arg};
use rocket::config::{Config, Environment};
use rocket::Data;
use rocket::State;
use rocket_slog::{SlogFairing, SyncLogger};
use sloggers::{
    terminal::{Destination, TerminalLoggerBuilder},
    types::Severity,
    Build,
};
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone)]
pub struct CachePath(String);

#[get("/<file..>")]
fn get(file: PathBuf, path: State<CachePath>, logger: SyncLogger) -> Option<CacheFile> {
    let together = format!("{}/{}", path.0, file.to_str().unwrap().to_string());
    info!(logger.get(), "formatted: {}", together);
    println!("{}", together);
    CacheFile::open(Path::new(together.as_str())).ok()
}

#[put("/<file..>", data = "<paste>")]
fn upload(
    paste: Data,
    file: PathBuf,
    path: State<CachePath>,
    logger: SyncLogger,
) -> io::Result<String> {
    let together = format!("{}/{}", path.0, file.to_str().unwrap().to_string());
    info!(logger.get(), "formatted: {}", together);
    println!("{}", together);
    let wfile = &mut File::create(together)?;
    let mut encoder = zstd::stream::Encoder::new(wfile, 5).unwrap();
    io::copy(&mut paste.open(), &mut encoder).unwrap();
    encoder.finish().unwrap();
    return Ok(file.to_str().unwrap().to_string());
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
        .mount("/", routes![upload, get])
        .launch();
}
