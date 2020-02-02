use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;

use cibo_util;
use cibo_util::config::ReadableDuration;
use storage::config::StorageConfig;

pub const DEFAULT_LISTENING_ADDR: &str = "127.0.0.1:20160";

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(with = "log_level_serde")]
    pub log_level: slog::Level,
    pub log_file: String,
    pub path: String,
    pub backtrace_dir: String,
    pub log_rotation_timespan: ReadableDuration,
    // Server listening address.
    pub addr: String,
    pub http_worker: usize,
    pub metric: MetricConfig,
    pub storage: StorageConfig,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            log_level: slog::Level::Info,
            log_file: "".to_owned(),
            backtrace_dir: "".to_owned(),
            path: "".to_owned(),
            log_rotation_timespan: ReadableDuration::hours(24),
            http_worker: 2,
            addr: DEFAULT_LISTENING_ADDR.to_owned(),
            metric: MetricConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self
    where
        P: fmt::Debug,
    {
        fs::File::open(&path)
            .map_err::<Box<dyn Error>, _>(|e| Box::new(e))
            .and_then(|mut f| {
                let mut s = String::new();
                f.read_to_string(&mut s)?;
                let c = ::toml::from_str(&s)?;
                Ok(c)
            })
            .unwrap_or_else(|e| {
                panic!(
                    "invalid auto generated configuration file {:?}, err {}",
                    path, e
                );
            })
    }
}

pub mod log_level_serde {
    use cibo_util::logger::{get_level_by_string, get_string_by_level};
    use serde::{
        de::{Error, Unexpected},
        Deserialize, Deserializer, Serialize, Serializer,
    };
    use slog::Level;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Level, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        get_level_by_string(&string)
            .ok_or_else(|| D::Error::invalid_value(Unexpected::Str(&string), &"a valid log level"))
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(value: &Level, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        get_string_by_level(*value).serialize(serializer)
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct MetricConfig {
    pub address: String,
}

impl Default for MetricConfig {
    fn default() -> MetricConfig {
        MetricConfig {
            address: "0.0.0.0:8091".to_owned(),
        }
    }
}
