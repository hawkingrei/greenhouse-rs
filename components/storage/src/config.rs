use std::default::Default;
use std::error::Error;

use threadpool::config::ThreadPoolConfig;

macro_rules! storage_config {
    ($struct_name:ident, $display_name:expr) => {
        #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
        #[serde(default)]
        #[serde(rename_all = "kebab-case")]
        pub struct $struct_name {
            pub cache_dir: String,
            pub threadpool: ThreadPoolConfig,
        }

        impl $struct_name {
            pub fn validate(&self) -> Result<(), Box<dyn Error>> {
                if self.cache_dir.is_empty() {
                    return Err("storage's cache_dir should be non-empty".into());
                }
                Ok(())
            }
        }
    };
}

storage_config!(StorageConfig, "storage");

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            cache_dir: "".to_string(),
            threadpool: Default::default(),
        }
    }
}
