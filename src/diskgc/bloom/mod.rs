pub mod spb;
pub mod store;

use chrono;
use chrono::offset::Local;
use chrono::prelude::*;
use crossbeam_channel::tick;
use crossbeam_channel::Receiver;
use protobuf::well_known_types::Timestamp;
use protobuf::Message;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;

use crate::config;
use crate::diskgc::bloom::spb::Record;
use crate::diskgc::bloom::store::new_gc_store;
use crate::diskgc::bloom::store::GcStore;
use crate::diskgc::lazy::get_entries;
use crate::util::bloomfilter::Bloom;
use crate::util::metrics;

const ITEMS_COUNT: usize = 500000;
const FP_P: f64 = 0.1;
const NUMBER_OF_BITS: u64 = 2396272;
const BITMAP_SIZE: usize = 299534;
const NUMBER_OF_HASH_FUNCTIONS: u32 = 4;

struct BloomEntry {
    bloom: Bloom<PathBuf>,
    total_put: u64,
}

pub struct Bloomgc {
    path: PathBuf,
    days: usize,
    receiver: Receiver<PathBuf>,
    bloomfilter: Bloom<PathBuf>,
    all_bloomfilter: Vec<BloomEntry>,
    store: GcStore,
}

impl Bloomgc {
    pub fn new(rx: Receiver<PathBuf>, p: PathBuf, days: usize) -> Bloomgc {
        let path = p.clone();
        let mut gc_file_path = p.clone();
        gc_file_path.pop();
        let mut store = new_gc_store(gc_file_path);
        let mut all_bloom: Vec<BloomEntry> = Vec::new();
        for get_bloom in store.get_all_bloom() {
            let bloom: Bloom<PathBuf> = Bloom::from_existing(
                get_bloom.data.as_slice(),
                NUMBER_OF_BITS,
                NUMBER_OF_HASH_FUNCTIONS,
                [(2749812374, 12341234), (574893759834, 1298374918234)],
            );
            let totalput = get_bloom.totalPut;
            all_bloom.push(BloomEntry {
                bloom: bloom,
                total_put: totalput,
            });
        }
        Bloomgc {
            path: path,
            receiver: rx,
            days: days,
            bloomfilter: Bloom::from_existing(
                &[0; BITMAP_SIZE],
                NUMBER_OF_BITS,
                NUMBER_OF_HASH_FUNCTIONS,
                [(2749812374, 12341234), (574893759834, 1298374918234)],
            ),
            store: store,
            all_bloomfilter: all_bloom,
        }
    }

    pub fn serve(&mut self) {
        let dt = chrono::Local::now();
        let ndt = chrono::Local
            .ymd(dt.year(), dt.month(), dt.day())
            .and_hms_milli(0, 0, 0, 0)
            - dt;
        let nt = tick(ndt.to_std().unwrap());
        let t = tick(Duration::from_secs(10));
        loop {
            select! {
                recv(self.receiver) -> path => {
                    if let Ok(p) = path { self.bloomfilter.set(&p) };
                },
                recv(t) -> _ => {
                    let mut rec = Record::new();
                    let mut now:Timestamp = Timestamp::new();
                    now.set_seconds(Local::now().timestamp());
                    rec.set_time(now);
                    rec.set_data(self.bloomfilter.bitmap());
                    rec.set_totalPut(config::total_put.load(Ordering::SeqCst) as u64);
                    let result = rec.write_to_bytes().unwrap();
                    self.store.save_today_bloom(result);
                },
                recv(nt) -> _ => {
                    let totalp = config::total_put.load(Ordering::SeqCst) as u64;
                    let bitmap = self.bloomfilter.bitmap();
                    let dt = chrono::Local::now();
                    let mut now:Timestamp = Timestamp::new();
                    now.set_seconds(chrono::Local.ymd(dt.year(), dt.month(), dt.day()-1).and_hms_milli(0, 0, 0, 0).timestamp());

                    let mut rec = Record::new();
                    rec.set_time(now);
                    rec.set_data(bitmap);
                    rec.set_totalPut(totalp);

                    if totalp > 200000 {
                        self.all_bloomfilter.push(BloomEntry{
                            bloom: self.bloomfilter.clone(),
                            total_put: totalp,
                        });
                    }

                    self.bloomfilter.clear();
                    config::total_put.swap(0,Ordering::SeqCst);

                    let ndt = chrono::Local.ymd(dt.year(), dt.month(), dt.day()+1).and_hms_milli(0, 0, 0, 0)-dt;
                    let nt = tick(ndt.to_std().unwrap());
                    self.store.append_to_all_bloom(rec);
                }
            }
        }
    }

    fn clear(&self) {
        println!("startgc {:?}", self.path.as_path());
        let entries = get_entries(self.path.as_path());
        for entry in entries.into_iter() {
            match fs::metadata(entry.path.as_path()) {
                Ok(meta) => {
                    if !(SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        - meta.ctime() as u64) as f64
                        / 3600.0
                        > 2.0 * 24.0
                    {
                        continue;
                    }
                    if !self.is_clear(&entry.path) {
                        continue;
                    }
                    match fs::remove_file(entry.path.as_path()) {
                        Ok(_) => {}
                        Err(e) => {
                            continue;
                        }
                    }
                    metrics::FilesEvicted.inc();
                    metrics::LastEvictedAccessAge.set(
                        (SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            - meta.ctime() as u64) as f64
                            / 3600.0,
                    );
                }
                Err(e) => {
                    continue;
                }
            };
        }
    }

    fn is_clear(&self, p: &PathBuf) -> bool {
        let ntime = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for element in self.all_bloomfilter.iter().rev().take(self.days) {
            if !element.bloom.check(&p) {
                return false;
            }
        }
        return true;
    }
}
