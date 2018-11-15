use std::cmp::Ordering;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

use crate::disk;
use crate::util::metrics;

#[derive(Eq)]
pub struct EntryInfo {
    path: PathBuf,
    last_access: i64,
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
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        let meta = match fs::metadata(p.clone()) {
            Ok(meta) => meta,
            Err(e) => {
                continue;
            }
        };
        if meta.is_file() {
            result.push(EntryInfo {
                path: p.to_path_buf(),
                last_access: meta.ctime(),
            })
        }
    }

    result.sort();
    return result;
}

#[derive(Clone)]
pub struct Lazygc {
    path: PathBuf,
    min_percent_block_free: f64,
    stop_percent_block: f64,
}

impl Lazygc {
    pub fn new<P: AsRef<Path>>(
        path: P,
        min_percent_block_free: f64,
        stop_percent_block: f64,
    ) -> Lazygc {
        Lazygc {
            path: path.as_ref().to_owned(),
            min_percent_block_free: min_percent_block_free,
            stop_percent_block: stop_percent_block,
        }
    }

    pub fn rocket(self) {
        match disk::get_disk_usage(self.path.as_path()) {
            Some((rate, _, _)) => {
                if rate < self.min_percent_block_free {
                    println!("startgc {:?}", self.path.as_path());
                    let entries = get_entries(self.path.as_path());
                    for entry in entries.into_iter() {
                        match fs::metadata(entry.path.as_path()) {
                            Ok(meta) => {
                                metrics::LastEvictedAccessAge.set(
                                    (SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs()
                                        - meta.ctime() as u64)
                                        as f64
                                        / 3600.0,
                                );
                            }
                            Err(e) => {
                                continue;
                            }
                        };
                        match fs::remove_file(entry.path.as_path()) {
                            Ok(_) => {}
                            Err(e) => {
                                continue;
                            }
                        }
                        metrics::FilesEvicted.inc();

                        match disk::get_disk_usage(self.path.as_path()) {
                            Some((rate, _, _)) => {
                                if rate >= self.stop_percent_block {
                                    break;
                                }
                            }
                            None => continue,
                        }
                    }
                    println!("end startgc");
                };
            }
            None => return,
        }
    }
}
