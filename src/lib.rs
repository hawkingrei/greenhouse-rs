#![feature(plugin)]
#![feature(type_ascription)]
#![plugin(rocket_codegen)]
#![feature(proc_macro_hygiene, decl_macro)]

extern crate rand;

#[macro_use]
extern crate quick_error;
extern crate brotli;
extern crate clap;
extern crate flate2;
extern crate lz4;
#[macro_use]
extern crate rocket;
extern crate rocket_slog;
extern crate sloggers;
extern crate snap;
extern crate zstd;
#[macro_use(debug)]
extern crate slog;

#[macro_use]
pub mod compression;
pub mod config;
pub mod file;
pub mod router;
pub mod util;
