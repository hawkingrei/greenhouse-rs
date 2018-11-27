#![feature(proc_macro_hygiene)]
#![feature(uniform_paths)]
#![recursion_limit = "128"]

extern crate greenhouse;
#[cfg(unix)]
extern crate nix;
#[cfg(unix)]
extern crate signal;
#[macro_use]
extern crate rocket;
extern crate env_logger;

use clap::{App, Arg};
use crossbeam_channel;
use futures::future::lazy;
use futures::Future;
use log::info;
use rocket::config::LoggingLevel;
use rocket::config::{Config, Environment, Limits};

use std::path::Path;
use std::path::PathBuf;
use std::{thread, time};
use tokio::runtime::Runtime;

use greenhouse::config::CachePath;
use greenhouse::disk::get_disk_usage_prom;
use greenhouse::diskgc::bloom::Bloomgc;
use greenhouse::diskgc::lazy;
use greenhouse::router;
use greenhouse::server::bloomgc::bloomgc_server;
use greenhouse::server::disk_usage_server::disk_usage_server;
use greenhouse::server::http::HttpServe;
use greenhouse::server::lazygc::lazygc_server;
use greenhouse::server::metric::MetricServer;

mod util;
use util::signal_handler;

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
    let _dir = matches.value_of("dir").unwrap_or("~/tmp/cache").to_owned();
    let _host = matches.value_of("host").unwrap_or("0.0.0.0").to_owned();;

    let _cache_port = matches
        .value_of("cache-port")
        .unwrap_or("8088")
        .parse::<u16>()
        .unwrap();
    let _metrics_port = matches
        .value_of("metrics-port")
        .unwrap_or("9090")
        .parse::<u16>()
        .unwrap();
    let metrics_dir = _dir.to_string().to_string();
    let gcpath = metrics_dir.clone();

    env_logger::init();
    let (tx, rx) = crossbeam_channel::unbounded::<PathBuf>();

    let mut http_server = HttpServe::new("0.0.0.0".to_string(), _cache_port, _dir, tx);
    let mut metrics_server = MetricServer::new(_metrics_port);

    let pathbuf = Path::new(&gcpath).to_path_buf();
    let ten_millis = time::Duration::from_secs(2);
    let mut disk_usage = disk_usage_server::new(ten_millis, pathbuf.clone());
    let mut lazygc = lazygc_server::new(pathbuf.clone(), 5.0, 20.0);
    let mut bloomgc = bloomgc_server::new(rx, pathbuf.clone(), 3);

    metrics_server.start().unwrap();
    disk_usage.start().unwrap();
    lazygc.start().unwrap();
    bloomgc.start().unwrap();
    http_server.start().unwrap();
    


    signal_handler::handle_signal();
    http_server.stop().unwrap();
    metrics_server.stop().unwrap();
    disk_usage.stop().unwrap();
    lazygc.stop().unwrap();
    bloomgc.stop().unwrap();
}
