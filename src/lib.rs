#![feature(plugin)]
#![feature(type_ascription)]
#![plugin(rocket_codegen)]
#![feature(proc_macro_hygiene, decl_macro)]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate quick_error;
extern crate brotli;
extern crate clap;
extern crate flate2;
extern crate rand;
#[macro_use]
extern crate lazy_static;
extern crate bit_vec;
extern crate hyper;
extern crate libc;
extern crate lz4;
extern crate rocket;
extern crate rocket_slog;
extern crate siphasher;
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
pub mod router;
pub mod util;