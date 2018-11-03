use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

use crate::disk;
use crate::util::metrics;

#[derive(Eq)]
pub struct EntryInfo {
    path: PathBuf,
    last_access: SystemTime,
}

impl Ord for EntryInfo {
    fn cmp(&self, other: &EntryInfo) -> Ordering {
        self.last_access.cmp(&other.last_access)
    }
}

impl PartialOrd for EntryInfo {
    fn partial_cmp(&self, other: &EntryInfo) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for EntryInfo {
    fn eq(&self, other: &EntryInfo) -> bool {
        self.last_access == other.last_access
    }
}

fn get_entries<P: AsRef<Path>>(path: P) -> Vec<EntryInfo> {
    let mut result: Vec<EntryInfo> = Vec::new();
    for entry in WalkDir::new(path).into_iter() {
        match entry {
            Ok(en) => {
                let meta = match fs::metadata(en.path()) {
                    Ok(meta) => meta,
                    Err(_) => continue,
                };
                if meta.is_dir() {
                    continue;
                }
                if let Ok(time) = meta.created() {
                    result.push(EntryInfo {
                        path: en.path().to_owned(),
                        last_access: time,
                    })
                }
            }
            Err(_) => continue,
        }
    }
    result.sort();
    return result;
}

#[derive(Clone)]
pub struct Lazygc {
    path: PathBuf,
    min_percent_block_free: f64,
}

impl Lazygc {
    pub fn new<P: AsRef<Path>>(path: P, min_percent_block_free: f64) -> Lazygc {
        Lazygc {
            path: path.as_ref().to_owned(),
            min_percent_block_free: min_percent_block_free,
        }
    }

    pub fn rocket(self) {
        match disk::get_disk_usage(self.path.as_path()) {
            Some((rate, _, _)) => {
                if rate < self.min_percent_block_free {
                    let entries = get_entries(self.path.as_path());
                    for entry in entries.into_iter() {
                        match fs::remove_file(entry.path.as_path()) {
                            Ok(_) => {}
                            Err(_) => continue,
                        }
                        metrics::FilesEvicted.inc();
                        match fs::metadata(entry.path.as_path()) {
                            Ok(meta) => {
                                if let Ok(time) = meta.created() {
                                    metrics::LastEvictedAccessAge.set(
                                        time.duration_since(SystemTime::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs()
                                            as f64,
                                    );
                                }
                            }
                            Err(_) => continue,
                        };
                        match disk::get_disk_usage(self.path.as_path()) {
                            Some((rate, _, _)) => {
                                if rate >= self.min_percent_block_free {
                                    break;
                                }
                            }
                            None => continue,
                        }
                    }
                };
            }
            None => return,
        }
    }
}
