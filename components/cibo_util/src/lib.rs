#![feature(rustc_private)]
#![feature(async_closure)]
#![recursion_limit = "128"]
#![allow(unused_imports)]

#[macro_use]
extern crate quick_error;
#[macro_use(slog_o, slog_error, slog_debug, slog_crit, slog_info, slog_warn)]
extern crate slog;
#[macro_use]
extern crate slog_global;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prometheus;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

#[macro_use]
pub mod macros;
pub mod collections;
pub mod config;
pub mod file;
pub mod future_pool;
pub mod logger;
pub mod metrics;
pub mod time;
pub mod worker;

static PANIC_WHEN_UNEXPECTED_KEY_OR_DATA: AtomicBool = AtomicBool::new(false);

pub fn panic_when_unexpected_key_or_data() -> bool {
    PANIC_WHEN_UNEXPECTED_KEY_OR_DATA.load(Ordering::SeqCst)
}

pub fn set_panic_when_unexpected_key_or_data(flag: bool) {
    PANIC_WHEN_UNEXPECTED_KEY_OR_DATA.store(flag, Ordering::SeqCst);
}

static PANIC_MARK: AtomicBool = AtomicBool::new(false);

pub fn set_panic_mark() {
    PANIC_MARK.store(true, Ordering::SeqCst);
}

pub fn panic_mark_is_on() -> bool {
    PANIC_MARK.load(Ordering::SeqCst)
}

pub fn panic_mark_file_path<P: AsRef<Path>>(data_dir: P) -> PathBuf {
    data_dir.as_ref().join(PANIC_MARK_FILE)
}

pub fn create_panic_mark_file<P: AsRef<Path>>(data_dir: P) {
    let file = panic_mark_file_path(data_dir);
    File::create(&file).unwrap();
}

pub const PANIC_MARK_FILE: &str = "panic_mark_file";

pub fn get_tag_from_thread_name() -> Option<String> {
    thread::current()
        .name()
        .and_then(|name| name.split("::").skip(1).last())
        .map(From::from)
}

/// Exit the whole process when panic.
pub fn set_panic_hook(panic_abort: bool, data_dir: &str) {
    use std::panic;
    use std::process;

    // HACK! New a backtrace ahead for caching necessary elf sections of this
    // tikv-server, in case it can not open more files during panicking
    // which leads to no stack info (0x5648bdfe4ff2 - <no info>).
    //
    // Crate backtrace caches debug info in a static variable `STATE`,
    // and the `STATE` lives forever once it has been created.
    // See more: https://github.com/alexcrichton/backtrace-rs/blob/\
    //           597ad44b131132f17ed76bf94ac489274dd16c7f/\
    //           src/symbolize/libbacktrace.rs#L126-L159
    // Caching is slow, spawn it in another thread to speed up.
    thread::Builder::new()
        .name(thd_name!("backtrace-loader"))
        .spawn(::backtrace::Backtrace::new)
        .unwrap();

    let data_dir = data_dir.to_string();
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info: &panic::PanicInfo<'_>| {
        use slog::Drain;
        if slog_global::borrow_global().is_enabled(::slog::Level::Error) {
            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match info.payload().downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<Any>",
                },
            };
            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");
            let loc = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()));
            let bt = backtrace::Backtrace::new();
            crit!("{}", msg;
                "thread_name" => name,
                "location" => loc.unwrap_or_else(|| "<unknown>".to_owned()),
                "backtrace" => format_args!("{:?}", bt),
            );
        } else {
            orig_hook(info);
        }

        // There might be remaining logs in the async logger.
        // To collect remaining logs and also collect future logs, replace the old one with a
        // terminal logger.
        if let Some(level) = log::max_level().to_level() {
            let drainer = logger::term_drainer();
            let _ = logger::init_log(
                drainer,
                logger::convert_log_level_to_slog_level(level),
                false, // Use sync logger to avoid an unnecessary log thread.
                false, // It is initialized already.
                vec![],
            );
        }

        // If PANIC_MARK is true, create panic mark file.
        if panic_mark_is_on() {
            create_panic_mark_file(data_dir.clone());
        }

        if panic_abort {
            process::abort();
        } else {
            process::exit(1);
        }
    }))
}

pub trait AssertSend: Send {}

pub trait AssertSync: Sync {}
