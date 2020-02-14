#![feature(async_closure)]
#![feature(map_first_last)]

#[macro_use]
extern crate slog_global;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

pub mod config;
mod lazygc;
mod metrics;

use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use async_compression::futures::write::{ZstdDecoder, ZstdEncoder};
use futures::AsyncWriteExt;
use threadpool::{Priority, ThreadPool};
use tokio::{fs, fs::File, io, prelude::*};

use crate::config::StorageConfig;
pub use crate::lazygc::LazygcServer;
pub use crate::lazygc::Lazygc;
pub use crate::metrics::*;

pub struct Storage {
    pool: Arc<ThreadPool>,
    basic_path: PathBuf,

    metric_handle: Option<thread::JoinHandle<()>>,
}

impl Storage {
    pub fn new(config: StorageConfig) -> Self {
        let path = PathBuf::from(config.cache_dir);
        if !path.as_path().exists() {
            std::fs::create_dir_all(path.as_path()).unwrap();
        }
        Storage {
            pool: Arc::new(ThreadPool::new(config.threadpool)),
            basic_path: path,
            metric_handle: None,
        }
    }

    fn priority_by_size(&self, size: u64) -> Priority {
        if size <= 1024 * 250 {
            Priority::HIGH
        } else if size <= 1024 * 1024 {
            Priority::NORMAL
        } else {
            Priority::LOW
        }
    }

    async fn priority_by_metadata(
        &self,
        path: impl AsRef<Path> + std::marker::Send + 'static,
    ) -> io::Result<Priority> {
        let size = fs::metadata(path).await?.len();
        Ok(self.priority_by_size(size))
    }

    pub async fn read(
        &self,
        path: impl AsRef<Path> + std::marker::Send + 'static,
    ) -> io::Result<Vec<u8>> {
        let p = self.basic_path.join(path);
        let priority = self.priority_by_metadata(p.clone()).await?;
        let future_fn = async move || -> io::Result<Vec<u8>> {
            let timer = STORAGE_READ_DURATION_SECONDS_HISTOGRAM_VEC.start_timer();
            let f = fs::read(p).await?;
            let mut e = ZstdDecoder::new(Vec::new());
            e.write_all(&f).await?;
            e.flush().await?;
            timer.observe_duration();
            Ok(e.into_inner())
        };
        match self.pool.spawn(future_fn(), priority) {
            Ok(middle) => match middle.await {
                Ok(data) => data,
                Err(e) => Err(io::Error::new(io::ErrorKind::WouldBlock, e)),
            },
            Err(e) => Err(io::Error::new(io::ErrorKind::WouldBlock, e)),
        }
    }

    pub async fn write(
        &self,
        data: Vec<u8>,
        path: impl AsRef<Path> + std::marker::Send + 'static,
    ) -> io::Result<()> {
        let p = self.basic_path.join(path);
        let priority = self.priority_by_size(data.len().try_into().unwrap());
        let future_fn = async move || -> io::Result<()> {
            let timer = STORAGE_WRITE_DURATION_SECONDS_HISTOGRAM_VEC.start_timer();
            let p_parent = p.as_path().parent().unwrap();
            if fs::metadata(p_parent).await.is_err() {
                fs::create_dir_all(p_parent).await?;
            }
            let mut e = ZstdEncoder::new(Vec::new(), 7);
            e.write_all(&data).await?;
            e.flush().await?;
            e.close().await?;
            let mut file = File::create(p).await?;
            file.write_all(&e.into_inner()).await?;
            timer.observe_duration();
            Ok(())
        };
        match self.pool.spawn(future_fn(), priority) {
            Ok(middle) => match middle.await {
                Ok(data) => data,
                Err(e) => Err(io::Error::new(io::ErrorKind::WouldBlock, e)),
            },
            Err(e) => Err(io::Error::new(io::ErrorKind::WouldBlock, e)),
        }
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        self.metric_handle.take();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
