use std::error::Error;

use cibo_util::config::ReadableSize;
use cibo_util::future_pool;

macro_rules! threadpool_config {
    ($struct_name:ident, $display_name:expr) => {
        #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
        #[serde(default)]
        #[serde(rename_all = "kebab-case")]
        pub struct $struct_name {
            pub high_concurrency: usize,
            pub normal_concurrency: usize,
            pub low_concurrency: usize,
            pub max_tasks_per_worker_high: usize,
            pub max_tasks_per_worker_normal: usize,
            pub max_tasks_per_worker_low: usize,
            pub stack_size: ReadableSize,
        }

        impl $struct_name {
            /// Builds configurations for low, normal and high priority pools.
            pub fn to_future_pool_configs(&self) -> Vec<future_pool::Config> {
                vec![
                    future_pool::Config {
                        workers: self.low_concurrency,
                        max_tasks_per_worker: self.max_tasks_per_worker_low,

                        stack_size: self.stack_size.0 as usize,
                    },
                    future_pool::Config {
                        workers: self.normal_concurrency,
                        max_tasks_per_worker: self.max_tasks_per_worker_normal,

                        stack_size: self.stack_size.0 as usize,
                    },
                    future_pool::Config {
                        workers: self.high_concurrency,
                        max_tasks_per_worker: self.max_tasks_per_worker_high,
                        stack_size: self.stack_size.0 as usize,
                    },
                ]
            }

            pub fn default_for_test() -> Self {
                Self {
                    high_concurrency: 2,
                    normal_concurrency: 2,
                    low_concurrency: 2,
                    max_tasks_per_worker_high: 2000,
                    max_tasks_per_worker_normal: 2000,
                    max_tasks_per_worker_low: 2000,
                    stack_size: ReadableSize::mb(1),
                }
            }

            pub fn validate(&self) -> Result<(), Box<dyn Error>> {
                if self.high_concurrency == 0 {
                    return Err(format!(
                        "threadpool.{}.high-concurrency should be > 0",
                        $display_name
                    )
                    .into());
                }
                if self.normal_concurrency == 0 {
                    return Err(format!(
                        "threadpool.{}.normal-concurrency should be > 0",
                        $display_name
                    )
                    .into());
                }
                if self.low_concurrency == 0 {
                    return Err(format!(
                        "threadpool.{}.low-concurrency should be > 0",
                        $display_name
                    )
                    .into());
                }
                if self.stack_size.0 < ReadableSize::mb(2).0 {
                    return Err(format!(
                        "threadpool.{}.stack-size should be >= 2mb",
                        $display_name
                    )
                    .into());
                }
                if self.max_tasks_per_worker_high <= 1 {
                    return Err(format!(
                        "threadpool.{}.max-tasks-per-worker-high should be > 1",
                        $display_name
                    )
                    .into());
                }
                if self.max_tasks_per_worker_normal <= 1 {
                    return Err(format!(
                        "threadpool.{}.max-tasks-per-worker-normal should be > 1",
                        $display_name
                    )
                    .into());
                }
                if self.max_tasks_per_worker_low <= 1 {
                    return Err(format!(
                        "threadpool.{}.max-tasks-per-worker-low should be > 1",
                        $display_name
                    )
                    .into());
                }
                Ok(())
            }
        }
    };
}

threadpool_config!(ThreadPoolConfig, "thumb");

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            high_concurrency: 16,
            normal_concurrency: 8,
            low_concurrency: 4,
            max_tasks_per_worker_high: 400,
            max_tasks_per_worker_normal: 200,
            max_tasks_per_worker_low: 10,
            stack_size: ReadableSize::mb(40),
        }
    }
}
