use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time;

use async_std::task;
use tokio::fs;

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

pub struct Lazygc<P: AsRef<Path>> {
    path: P,
    min_percent_block_free: f64,
    stop_percent_block: f64,

    entry_map: BTreeMap<EntryInfo, u64>,
    entry_total_size: u64,
}

impl<P: AsRef<Path>> Lazygc<P> {
    pub fn new(path: P, min_percent_block_free: f64, stop_percent_block: f64) -> Lazygc<P> {
        Lazygc {
            path,
            min_percent_block_free,
            stop_percent_block,
            entry_map: BTreeMap::new(),
            entry_total_size: 0,
        }
    }

    pub async fn start(&mut self) {
        if DISK_USED.get() * 100.0 / DISK_TOTAL.get() > self.min_percent_block_free {
            self.get().await;
            for (key, _) in self.entry_map.iter() {
                fs::remove_file(&key.path).await;
            }
            self.entry_map.clear();
        }
    }

    async fn get(&mut self) {
        for entry in WalkDir::new(&self.path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            let meta = match fs::metadata(&p).await {
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

    pub fn start_cleaner(&mut self) {
        task::block_on(async move {
            loop {
                thread::sleep(time::Duration::from_millis(1024 * 60));
                self.start().await;
            }
        });
    }
}
