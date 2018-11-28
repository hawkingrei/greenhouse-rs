#![crate_type = "lib"]
#![feature(plugin)]
#![feature(type_ascription)]
#![feature(custom_attribute)]
#![feature(proc_macro_hygiene, decl_macro)]
#![feature(optin_builtin_traits)]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![recursion_limit = "128"]
#![feature(uniform_paths)]

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
extern crate chrono;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate serde;
#[cfg_attr(not(test), macro_use(slog_info))]
#[cfg_attr(test, macro_use(slog_info))]
extern crate slog;
extern crate slog_async;
extern crate slog_scope;
extern crate slog_stdlog;
extern crate slog_term;
extern crate snap;
extern crate tempfile;
extern crate walkdir;
extern crate zstd;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate spin;

#[macro_use]
pub mod util;
#[macro_use]
pub mod compression;
pub mod config;
pub mod disk;
pub mod diskgc;
pub mod env;
pub mod router;
pub mod server;
