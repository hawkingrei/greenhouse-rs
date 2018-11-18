#![crate_type = "lib"]
#![feature(plugin)]
#![feature(int_to_from_bytes)]
#![feature(type_ascription)]
#![feature(custom_attribute)]
#![feature(proc_macro_hygiene, decl_macro)]
#![feature(optin_builtin_traits)]
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
extern crate log;
extern crate protobuf;
#[cfg_attr(not(test), macro_use(slog_info))]
#[cfg_attr(test, macro_use(slog_info))]
extern crate slog;
extern crate sloggers;
extern crate snap;
extern crate tempfile;
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
