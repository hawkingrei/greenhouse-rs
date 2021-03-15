use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::{fs, io, thread};

use crossbeam::queue::ArrayQueue;

use crate::config::StorageConfig;

pub use self::write_file::WriteFile;
use crate::background::write_file::WriteFileTask;

mod write_file;

pub const WRITE_FILE_SIZE: usize = 8 * 1024;

lazy_static! {
    pub static ref WRITE_FILE_BUFFER: ArrayQueue<WriteFile> = ArrayQueue::new(WRITE_FILE_SIZE);
}

pub struct Background {
    basic_path: PathBuf,
    workers: Vec<JoinHandle<()>>,
}

impl Background {
    pub fn new(config: &StorageConfig) -> Background {
        let path = PathBuf::from(&config.cache_dir);
        Background {
            workers: vec![],
            basic_path: path,
        }
    }

    pub fn start(&mut self) {
        self.start_write_file();
    }

    pub fn start_write_file(&mut self) {
        for n in 0..2 {
            let write_file_task = WriteFileTask::new(self.basic_path.clone());
            let t = thread::Builder::new()
                .name(thd_name!(format!("deal_write_file_{}", n)))
                .spawn(move || loop {
                    if let Err(e) = write_file_task.deal_write_file() {
                        error!("write_file_batch_error";  "error" => ?e);
                    }
                    thread::yield_now();
                })
                .unwrap();
            self.workers.push(t);
        }
    }

    pub fn shutdown(&mut self) {
        for h in self.workers.drain(..) {
            debug!("waiting for {}", h.thread().name().unwrap());
            if let Err(e) = h.join() {
                error!("failed to join worker thread: {:?}", e);
            }
        }
    }
}
