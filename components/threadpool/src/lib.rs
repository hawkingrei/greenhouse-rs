#![feature(trivial_bounds)]

#[macro_use]
extern crate serde_derive;

pub mod config;

use cibo_util::future_pool::{Builder, Config, ErrorPoolFull, FuturePool};
use futures::Future;
use tokio::task::JoinHandle;

pub use crate::config::ThreadPoolConfig;

// Optimize level
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Priority {
    HIGH,
    NORMAL,
    LOW,
}

#[derive(Clone)]
pub struct ThreadPool {
    pool_high: FuturePool,
    pool_normal: FuturePool,
    pool_low: FuturePool,
}

impl ThreadPool {
    pub fn new(config: ThreadPoolConfig) -> Self {
        let names = vec!["pool-low", "pool-normal", "pool-high"];
        let configs: Vec<Config> = config.to_future_pool_configs();
        let mut pools: Vec<FuturePool> = configs
            .into_iter()
            .zip(names)
            .map(|(config, name)| Builder::from_config(config).name_prefix(name).build())
            .collect();
        let pool_high = pools.remove(2);
        let pool_normal = pools.remove(1);
        let pool_low = pools.remove(0);
        Self {
            pool_high,
            pool_normal,
            pool_low,
        }
    }

    pub fn spawn<T>(
        &self,
        future_fn: T,
        pri: Priority,
    ) -> Result<JoinHandle<T::Output>, ErrorPoolFull>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        match pri {
            Priority::HIGH => self.pool_high.spawn(future_fn),
            Priority::NORMAL => self.pool_normal.spawn(future_fn),
            Priority::LOW => self.pool_low.spawn(future_fn),
        }
    }
}
