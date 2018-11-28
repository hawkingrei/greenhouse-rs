use log;
use std::sync::atomic::AtomicUsize;

#[derive(Clone)]
pub struct CachePath(pub String);

lazy_static! {
    pub static ref total_put: AtomicUsize = AtomicUsize::new(0);
}

pub mod log_level_serde {
    use crate::util::logger::{get_level_by_string, get_string_by_level};
    use log::Level;
    use serde::{
        de::{Error, Unexpected},
        Deserialize, Deserializer, Serialize, Serializer,
    };

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Level, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        get_level_by_string(&string)
            .ok_or_else(|| D::Error::invalid_value(Unexpected::Str(&string), &"a valid log level"))
    }

    #[cfg_attr(feature = "cargo-clippy", allow(trivially_copy_pass_by_ref))]
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
pub struct HttpServeConfig {
    pub path: String,
    pub metric_port: u32,
    pub cache_port: u32,
}

impl Default for HttpServeConfig {
    fn default() -> HttpServeConfig {
        HttpServeConfig {
            path: "".to_owned(),
            metric_port: 9092,
            cache_port: 8088,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct GreenhouseConfig {
    #[serde(with = "log_level_serde")]
    pub log_level: log::Level,
    pub log_file: String,
}

impl Default for GreenhouseConfig {
    fn default() -> GreenhouseConfig {
        GreenhouseConfig {
            log_level: log::Level::Info,
            log_file: "".to_owned(),
        }
    }
}
