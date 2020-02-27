use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::thread;
use std::time;

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

#[derive(Clone)]
pub struct Lazygc {
    path: PathBuf,
    min_percent_block_free: f64,
    stop_percent_block: f64,

    entry_map: BTreeMap<EntryInfo, u64>,
    entry_total_size: u64,
    total_size: u64,
}

impl Lazygc {
    pub fn new(path: PathBuf, min_percent_block_free: f64, stop_percent_block: f64) -> Lazygc {
        Lazygc {
            path,
            min_percent_block_free,
            stop_percent_block,
            entry_map: BTreeMap::new(),
            entry_total_size: 0,
            total_size: 0,
        }
    }

    pub fn start(&mut self) {
        if let Some((_, bytes_free, bytes_used)) = get_disk_usage(self.path.clone()) {
            self.total_size = bytes_free + bytes_used;
            if bytes_used as f64 / bytes_free as f64 > self.min_percent_block_free {
                info!("start to clearn");
                self.get();
                for (key, _) in self.entry_map.iter() {
                    info!("rm file"; "file" => &key.path.to_str());
                    std::fs::remove_file(&key.path);
                }
                self.entry_map.clear();
            }
        }
    }

    fn get(&mut self) {
        for entry in WalkDir::new(&self.path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            let meta = match std::fs::metadata(&p) {
                Ok(meta) => meta,
                Err(_) => {
                    info!("get continue");
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
        let stop = (self.min_percent_block_free - self.stop_percent_block) * self.total_size as f64;
        while (stop as u64) < self.entry_total_size {
            info!("rev map {} {}", stop, self.entry_total_size);
            let (key, value) = self.entry_map.first_key_value().unwrap();
            let key_copy = key.clone();
            self.entry_total_size -= value;
            self.entry_map.remove(&key_copy);
        }
    }
}

pub struct LazygcServer {
    lazygc_handle: Option<thread::JoinHandle<()>>,

    path: PathBuf,
    min_percent_block_free: f64,
    stop_percent_block: f64,
}

impl LazygcServer {
    pub fn new(
        path: PathBuf,
        min_percent_block_free: f64,
        stop_percent_block: f64,
    ) -> LazygcServer {
        return LazygcServer {
            lazygc_handle: None,
            path,
            min_percent_block_free,
            stop_percent_block,
        };
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        let builder = thread::Builder::new().name("lazy-service".to_string());
        let gc = Lazygc::new(
            self.path.clone(),
            self.min_percent_block_free,
            self.stop_percent_block,
        );
        let ten_millis = time::Duration::from_secs(10);
        let h = builder.spawn(move || loop {
            info!("lazy gc start");
            gc.clone().start();
            thread::sleep(ten_millis);
        })?;
        self.lazygc_handle = Some(h);
        Ok(())
    }
}

impl Drop for LazygcServer {
    fn drop(&mut self) {
        info!("stop cleaner server");
        if let Some(h) = self.lazygc_handle.take() {
            h.join().unwrap();
        };
    }
}
