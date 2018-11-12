#![crate_type = "lib"]
#![feature(plugin)]
#![feature(type_ascription)]
#![feature(custom_attribute)]
#![feature(proc_macro_hygiene, decl_macro)]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![recursion_limit = "128"]

extern crate crossbeam;
#[macro_use]
extern crate crossbeam_channel;
extern crate core_affinity;
extern crate tokio_timer;
#[macro_use]
extern crate quick_error;
extern crate brotli;
extern crate clap;
extern crate flate2;
extern crate rand;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate lz4;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate log;
#[cfg_attr(not(test), macro_use(slog_o, slog_info, slog_kv))]
#[cfg_attr(
    test,
    macro_use(
        slog_info,
        slog_o,
        slog_kv,
        slog_crit,
        slog_log,
        slog_record,
        slog_b,
        slog_record_static
    )
)]
extern crate slog;
extern crate sloggers;
extern crate snap;
extern crate walkdir;
extern crate zstd;

#[macro_use]
pub mod compression;
pub mod config;
pub mod disk;
pub mod diskgc;
pub mod env;
pub mod router;
pub mod util;
