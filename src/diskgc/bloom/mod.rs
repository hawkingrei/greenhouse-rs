pub mod spb;
pub mod store;

use chrono;
use chrono::offset::Local;
use chrono::prelude::*;
use crossbeam_channel::tick;
use crossbeam_channel::Receiver;
use log::info;
use protobuf::well_known_types::Timestamp;
use protobuf::Message;
use spin;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use crate::config;
use crate::diskgc::bloom::spb::Record;
use crate::diskgc::bloom::store::new_gc_store;
use crate::diskgc::bloom::store::GcStore;
use crate::diskgc::lazy::get_entries;
use crate::util::bloomfilter::Bloom;
use crate::util::metrics;

const ITEMS_COUNT: usize = 500_000;
const FP_P: f64 = 0.1;
const NUMBER_OF_BITS: u64 = 2_396_272;
const BITMAP_SIZE: usize = 299_534;
const NUMBER_OF_HASH_FUNCTIONS: u32 = 4;

struct BloomEntry {
    bloom: Bloom<PathBuf>,
    total_put: u64,
}

pub struct Bloomgc {
    path: PathBuf,
    days: usize,
    receiver: Receiver<PathBuf>,
    bloomfilter: Arc<spin::Mutex<Bloom<PathBuf>>>,
    all_bloomfilter: Box<Vec<BloomEntry>>,
    store: GcStore,
    next_time: chrono::DateTime<chrono::Local>,
}

impl Bloomgc {
    pub fn get_next_time(&self) -> chrono::DateTime<chrono::Local> {
        self.next_time
    }

    pub fn set_next_time(&mut self, nt: chrono::DateTime<chrono::Local>) {
        self.next_time = nt;
    }

    pub fn new(rx: Receiver<PathBuf>, p: PathBuf, days: usize) -> Bloomgc {
        let path = p.clone();
        let mut gc_file_path = p.clone();
        gc_file_path.pop();
        let mut store = new_gc_store(gc_file_path);
        let mut all_bloom: Vec<BloomEntry> = Vec::new();
        let (all, today) = store.get_all();
        let bloomfilter: Arc<spin::Mutex<Bloom<PathBuf>>> = match today {
            Ok(r) => Arc::new(spin::Mutex::new(Bloom::from_existing(
                &r.as_slice(),
                NUMBER_OF_BITS,
                NUMBER_OF_HASH_FUNCTIONS,
                [(2749812374, 12341234), (574893759834, 1298374918234)],
            ))),
            Err(_) => Arc::new(spin::Mutex::new(Bloom::from_existing(
                &[0; BITMAP_SIZE],
                NUMBER_OF_BITS,
                NUMBER_OF_HASH_FUNCTIONS,
                [(2749812374, 12341234), (574893759834, 1298374918234)],
            ))),
        };
        for get_bloom in all {
            info!("{}", "bgc new start");
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
        let dt = chrono::Local::now();
        let ndt = chrono::Local
            .ymd(dt.year(), dt.month(), dt.day() + 1)
            .and_hms_milli(0, 0, 0, 0);
        let bloomfilter: Arc<spin::Mutex<Bloom<PathBuf>>> =
            Arc::new(spin::Mutex::new(Bloom::from_existing(
                &[0; BITMAP_SIZE],
                NUMBER_OF_BITS,
                NUMBER_OF_HASH_FUNCTIONS,
                [(2749812374, 12341234), (574893759834, 1298374918234)],
            )));
        Bloomgc {
            path: path,
            receiver: rx,
            days: days,
            bloomfilter: bloomfilter,
            store: store,
            all_bloomfilter: Box::new(all_bloom),
            next_time: ndt,
        }
    }

    fn append_today_bloom(&mut self) {
        let totalp = config::total_put.load(Ordering::SeqCst) as u64;
        let mut guard = self.bloomfilter.lock();
        let copy_bloomfilter = guard.clone();
        guard.clear();
        drop(guard);
        config::total_put.swap(0, Ordering::SeqCst);
        let bitmap = copy_bloomfilter.bitmap();
        let dt = chrono::Local::now();
        let mut now: Timestamp = Timestamp::new();
        now.set_seconds(
            chrono::Local
                .ymd(dt.year(), dt.month(), dt.day() - 1)
                .and_hms_milli(0, 0, 0, 0)
                .timestamp(),
        );

        let mut rec = Record::new();
        rec.set_time(now);
        rec.set_data(bitmap);
        rec.set_totalPut(totalp);
        self.store.append_to_all_bloom(rec).unwrap();
        if totalp > 200000 {
            let mut guard = self.bloomfilter.lock();
            self.all_bloomfilter.push(BloomEntry {
                bloom: copy_bloomfilter,
                total_put: totalp,
            });
        }
        info!("append today bloom")
    }

    fn clear(&self) {
        info!("startgc {:?}", self.path.as_path());
        let entries = get_entries(self.path.as_path());
        for entry in entries.into_iter() {
            match fs::metadata(entry.path.as_path()) {
                Ok(meta) => {
                    if !((SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        - meta.ctime() as u64) as f64
                        / 3600.0
                        > 24.0)
                    {
                        continue;
                    }
                    if self.is_clear(&entry.path) {
                        continue;
                    }
                    match fs::remove_file(entry.path.as_path()) {
                        Ok(_) => {}
                        Err(_) => {
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

    pub fn serve(&mut self) {
        let t = tick(Duration::from_secs(5));
        loop {
            select! {
                recv(self.receiver) -> path => {
                    if let Ok(p) = path {
                        let mut guard = self.bloomfilter.lock();
                        guard.set(&p);
                        drop(guard);
                    };
                },
                recv(t) -> _ => {
                    let mut rec = Record::new();
                    let mut now:Timestamp = Timestamp::new();
                    now.set_seconds(Local::now().timestamp());
                    rec.set_time(now);
                    let mut guard = self.bloomfilter.lock();
                    rec.set_data(guard.bitmap());
                    drop(guard);
                    rec.set_totalPut(config::total_put.load(Ordering::SeqCst) as u64);
                    let result = rec.write_to_bytes().unwrap();
                    self.store.save_today_bloom(result).unwrap();
                    info!("{}","save today bloom");
                },
                default(Duration::from_secs(3)) => {
                    if chrono::Local::now() > self.get_next_time() {
                        self.append_today_bloom();
                        self.clear();

                        let dt = chrono::Local::now();
                        self.set_next_time(
                            chrono::Local
                                .ymd(dt.year(), dt.month(), dt.day() + 1)
                                .and_hms_milli(0, 0, 0, 0),
                        );
                    }
                }
            }
        }
    }

    fn is_clear(&self, p: &PathBuf) -> bool {
        let ntime = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if self.all_bloomfilter.len() < self.days {
            return false;
        }
        let mut is_existed = false;
        for element in self.all_bloomfilter.iter().rev().take(self.days) {
            if !is_existed && element.bloom.check(&p) {
                is_existed = true;
            }
        }
        return !is_existed;
    }
}

#[test]
fn test_bloom_tick() {
    let dt = chrono::Local.ymd(2018, 11, 23).and_hms_milli(0, 0, 0, 0);
    let ndt = chrono::Local
        .ymd(dt.year(), dt.month(), dt.day() + 1)
        .and_hms_milli(0, 0, 0, 0)
        - dt;
    let nt = ndt.to_std().unwrap();
    assert_eq!(nt, Duration::from_secs(86400));
}
