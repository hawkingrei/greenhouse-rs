use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::{io, thread};

use crossbeam::queue::ArrayQueue;
use tokio::runtime;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio::fs::File;
use tokio::fs;

use threadpool::ThreadPool;

use crate::background::write_file::WriteFileTask;
use crate::config::StorageConfig;

pub use self::write_file::WriteFile;

mod write_file;

pub const WRITE_FILE_SIZE: usize = 8 * 1024;

lazy_static! {
    pub static ref WRITE_FILE_BUFFER: ArrayQueue<WriteFile> = ArrayQueue::new(WRITE_FILE_SIZE);
}

pub struct Background {
    basic_path: PathBuf,
    writing_pool: Runtime,
    workers: Vec<JoinHandle<()>>,
}

impl Background {
    pub fn new(config: &StorageConfig) -> Background {
        let path = PathBuf::from(&config.cache_dir);
        let writing_pool = runtime::Builder::new()
            .threaded_scheduler()
            .build()
            .unwrap();
        Background {
            writing_pool,
            workers: vec![],
            basic_path: path,
        }
    }

    pub fn start(&mut self) {
        self.start_write_file();
    }

    pub fn start_write_file(&mut self) {
        for n in 0..8 {
            let write_file_task = WriteFileTask::new(self.basic_path.clone());
            let t = self
                .writing_pool
                .spawn(async move || loop {
                    if let Err(e) = write_file_task.deal_write_file().await {
                        error!("write_file_batch_error";  "error" => ?e);
                    }
                });
            self.workers.push(t);
        }
    }

    pub fn shutdown(&mut self) {
        for _ in self.workers.drain(..) {}
    }
}
