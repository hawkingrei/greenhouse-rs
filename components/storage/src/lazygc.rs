use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::thread;
use std::time;

use tokio::fs;
use tokio::task::JoinHandle;
use walkdir::WalkDir;

use crate::metrics::*;

#[derive(Eq, Clone)]
pub struct EntryInfo {
    pub path: PathBuf,
    pub last_access: i64,
}

impl Ord for EntryInfo {
    fn cmp(&self, other: &EntryInfo) -> Ordering {
        let order = self.last_access.cmp(&other.last_access);
        if order == Ordering::Equal {
            return self.path.cmp(&other.path);
        };
        order
    }
}

impl PartialOrd for EntryInfo {
    fn partial_cmp(&self, other: &EntryInfo) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for EntryInfo {
    fn eq(&self, other: &EntryInfo) -> bool {
        self.last_access == other.last_access && self.path == other.path
    }
}

pub struct Lazygc {
    path: PathBuf,
    min_percent_block_free: f64,
    stop_percent_block: f64,

    entry_map: BTreeMap<EntryInfo, u64>,
    entry_total_size: u64,
}

impl Lazygc {
    pub fn new(path: PathBuf, min_percent_block_free: f64, stop_percent_block: f64) -> Lazygc {
        Lazygc {
            path,
            min_percent_block_free,
            stop_percent_block,
            entry_map: BTreeMap::new(),
            entry_total_size: 0,
        }
    }

    pub async fn start(&mut self) {
        info!(
            "DISK_USED:{} DISK_TOTAL:{} min_percent_block_free:{}",
            DISK_USED.get(),
            DISK_TOTAL.get(),
            self.min_percent_block_free
        );
        if DISK_USED.get() / DISK_TOTAL.get() > self.min_percent_block_free {
            info!("start to clearn");
            self.get().await;
            for (key, _) in self.entry_map.iter() {
                info!("rm file"; "file" => &key.path.to_str());
                std::fs::remove_file(&key.path);
            }
            self.entry_map.clear();
        }
    }

    async fn get(&mut self) {
        for entry in WalkDir::new(&self.path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            let meta = match std::fs::metadata(&p) {
                Ok(meta) => meta,
                Err(_) => {
                    continue;
                }
            };
            if meta.is_file() {
                self.entry_map.insert(
                    EntryInfo {
                        path: p.to_path_buf(),
                        last_access: meta.ctime(),
                    },
                    meta.size(),
                );
                self.entry_total_size += meta.size();
            }
            self.clean_map();
        }
    }

    fn clean_map(&mut self) {
        let stop = self.stop_percent_block * self.entry_total_size as f64;
        while (stop as u64) < self.entry_total_size {
            let (key, value) = self.entry_map.first_key_value().unwrap();
            let key_copy = key.clone();
            self.entry_total_size -= value;
            self.entry_map.remove(&key_copy);
        }
    }
}

pub struct LazygcServer {}

impl LazygcServer {
    pub async fn new(
        path: PathBuf,
        min_percent_block_free: f64,
        stop_percent_block: f64,
    ) -> JoinHandle<()> {
        info!("start clearner");
        use tokio::runtime::Builder as TokioBuilder;
        let rt = TokioBuilder::new()
            .basic_scheduler()
            .core_threads(1)
            .thread_stack_size(3 * 1024 * 1024)
            .thread_name("lazygc")
            .enable_io()
            .build()
            .unwrap();
        let h = rt.spawn(async move {
            let mut gc = Lazygc::new(path, min_percent_block_free, stop_percent_block);
            loop {
                gc.start().await;
                thread::sleep(time::Duration::from_millis(1024));
            }
        });
        h
    }
}

impl Drop for LazygcServer {
    fn drop(&mut self) {
        info!("stop cleaner server");
    }
}
