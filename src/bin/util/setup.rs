use std::io;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

use cibo_util::logger;
use greenhouse::config::Config;

// A workaround for checking if log is initialized.
pub static LOG_INITIALIZED: AtomicBool = AtomicBool::new(false);

macro_rules! fatal {
    ($lvl:expr, $($arg:tt)+) => ({
        if LOG_INITIALIZED.load(Ordering::SeqCst) {
            error!($lvl, $($arg)+);
        } else {
            eprintln!($lvl, $($arg)+);
        }
        process::exit(1)
    })
}

pub fn initial_logger(config: &Config) {
    let log_rotation_timespan =
        chrono::Duration::from_std(config.log_rotation_timespan.clone().into())
            .expect("config.log_rotation_timespan is an invalid duration.");
    if config.log_file.is_empty() {
        let drainer = slog_json::Json::default(io::stdout());
        // use async drainer and init std log.
        logger::init_log(drainer, config.log_level, true, true).unwrap_or_else(|e| {
            fatal!("failed to initialize log: {}", e);
        });
    } else {
        let drainer =
            logger::file_drainer(&config.log_file, log_rotation_timespan).unwrap_or_else(|e| {
                fatal!(
                    "failed to initialize log with file {}: {}",
                    config.log_file,
                    e
                );
            });
        // use async drainer and init std log.
        logger::init_log(drainer, config.log_level, true, true).unwrap_or_else(|e| {
            fatal!("failed to initialize log: {}", e);
        });
    };
    LOG_INITIALIZED.store(true, Ordering::SeqCst);
}
