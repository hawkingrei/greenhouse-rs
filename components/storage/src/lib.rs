#![feature(async_closure)]
#![feature(map_first_last)]

#[macro_use]
extern crate cibo_util;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog_global;

mod background;
pub mod config;
mod lazygc;
mod metrics;

use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use futures::AsyncWriteExt;
use threadpool::{Priority, ThreadPool};
use tokio::{fs, fs::File, io, prelude::*};
use zstd;

use crate::background::{Background, WriteFile, WRITE_FILE_BUFFER};
use crate::config::StorageConfig;
pub use crate::lazygc::Lazygc;
pub use crate::lazygc::LazygcServer;
pub use crate::metrics::*;

pub struct Storage {
    reading_pool: Arc<ThreadPool>,
    writing_pool: Arc<ThreadPool>,
    basic_path: PathBuf,
    background: Background,

    metric_handle: Option<thread::JoinHandle<()>>,
}

impl Storage {
    pub fn new(config: StorageConfig) -> Self {
        let path = PathBuf::from(&config.cache_dir);
        if !path.as_path().exists() {
            std::fs::create_dir_all(path.as_path()).unwrap();
        }
        let mut background = Background::new(&config);
        background.start();
        Storage {
            reading_pool: Arc::new(ThreadPool::new(config.reading_threadpool)),
            writing_pool: Arc::new(ThreadPool::new(config.writing_threadpool)),
            basic_path: path,
            metric_handle: None,
            background,
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
            let mut read_data = vec![];
            match zstd::stream::copy_decode(&mut &*f.as_slice(), &mut read_data) {
                Ok(()) => (),
                Err(e) => return Err(io::Error::new(io::ErrorKind::WouldBlock, e)),
            }
            timer.observe_duration();
            Ok(read_data)
        };
        match self.reading_pool.spawn(future_fn(), priority) {
            Ok(middle) => match middle.await {
                Ok(data) => data,
                Err(e) => Err(io::Error::new(io::ErrorKind::WouldBlock, e)),
            },
            Err(e) => Err(io::Error::new(io::ErrorKind::WouldBlock, e)),
        }
    }

    pub async fn delete(
        &self,
        path: impl AsRef<Path> + std::marker::Send + 'static,
    ) -> io::Result<()> {
        let p = self.basic_path.join(path);
        fs::remove_file(p).await?;
        Ok(())
    }

    pub async fn write(
        &self,
        data: Vec<u8>,
        path: impl AsRef<Path> + std::marker::Send + 'static,
    ) -> io::Result<()> {
        let timer = STORAGE_WRITE_DURATION_SECONDS_HISTOGRAM_VEC.start_timer();
        let wf = WriteFile::new(data, path.as_ref().to_path_buf());
        stat_err!(
            WRITE_FILE_BUFFER.push(wf),
            WRITE_FILE_BUFFER_OVERLIMIT.inc()
        );
        timer.observe_duration();
        Ok(())
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        self.metric_handle.take();
        self.background.shutdown();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
