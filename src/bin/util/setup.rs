use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Local;
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

// TODO: There is a very small chance that duplicate files will be generated if there are
// a lot of logs written in a very short time. Consider rename the rotated file with a version
// number while rotate by size.
fn rename_by_timestamp(path: &Path) -> io::Result<PathBuf> {
    let mut new_path = path.to_path_buf().into_os_string();
    new_path.push(format!("{}", Local::now().format("%Y-%m-%d-%H:%M:%S%.f")));
    Ok(PathBuf::from(new_path))
}

pub fn initial_logger(config: &Config) {
    if config.log_file.is_empty() {
        let drainer = slog_json::Json::default(io::stdout());
        // use async drainer and init std log.
        logger::init_log(drainer, config.log_level, true, true, vec![]).unwrap_or_else(|e| {
            fatal!("failed to initialize log: {}", e);
        });
    } else {
        let drainer = logger::file_drainer(
            &config.log_file,
            config.log_rotation_timespan,
            config.log_rotation_size,
            rename_by_timestamp,
        )
        .unwrap_or_else(|e| {
            fatal!(
                "failed to initialize log with file {}: {}",
                config.log_file,
                e
            );
        });
        // use async drainer and init std log.
        logger::init_log(drainer, config.log_level, true, true, vec![]).unwrap_or_else(|e| {
            fatal!("failed to initialize log: {}", e);
        });
    };
    LOG_INITIALIZED.store(true, Ordering::SeqCst);
}
