#[macro_use]
extern crate slog_global;

#[cfg(unix)]
#[macro_use]
mod util;

use crate::util::setup::initial_logger;


use cibo_util;
use clap::{App, Arg};
use greenhouse::config::Config;
use greenhouse::route;

fn main() {
    let matches = App::new("greenhouse")
        .author("hawkingrei <hawkingrei@gmail.com>")
        .arg(
            Arg::with_name("config")
                .short("C")
                .long("config")
                .value_name("FILE")
                .help("Set the configuration file")
                .takes_value(true),
        )
        .get_matches();

    let cfg = matches
        .value_of("config")
        .map_or_else(Config::default, |path| Config::from_file(&path));

    // Sets the global logger ASAP.
    // It is okay to use the config w/o `validate()`,
    // because `initial_logger()` handles various conditions.
    initial_logger(&cfg);
    cibo_util::set_panic_hook(false, &cfg.backtrace_dir);
    info!(
        "using config";
        "config" => serde_json::to_string(&cfg).unwrap(),
    );
    route::run(cfg);
}
