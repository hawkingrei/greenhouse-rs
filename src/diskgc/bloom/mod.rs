pub mod spb;
pub mod store;

use chrono;
use chrono::offset::Local;
use chrono::prelude::*;
use crossbeam_channel::tick;
use crossbeam_channel::Receiver;
use protobuf::well_known_types::Timestamp;
use protobuf::Message;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::config;
use crate::diskgc::bloom::spb::Record;
use crate::diskgc::bloom::store::gc_store;
use crate::diskgc::bloom::store::new_gc_store;
use crate::util::bloomfilter::Bloom;

const items_count: usize = 500000;
const fp_p: f64 = 0.1;
const number_of_bits: u64 = 2396272;
const bitmap_size: usize = 299534;
const number_of_hash_functions: u32 = 4;

struct bloom_entry {
    bloom: Bloom<PathBuf>,
    total_put: u64,
}

pub struct bloomgc {
    receiver: Receiver<PathBuf>,
    bloomfilter: Bloom<PathBuf>,
    all_bloomfilter: Vec<bloom_entry>,
    store: gc_store,
}

impl bloomgc {
    pub fn new(rx: Receiver<PathBuf>, p: PathBuf) -> bloomgc {
        let mut store = new_gc_store(p);
        let mut all_bloom: Vec<bloom_entry> = Vec::new();
        for get_bloom in store.get_all_bloom() {
            let bloom: Bloom<PathBuf> = Bloom::from_existing(
                get_bloom.data.as_slice(),
                number_of_bits,
                number_of_hash_functions,
                [(2749812374, 12341234), (574893759834, 1298374918234)],
            );
            let totalput = get_bloom.totalPut;
            all_bloom.push(bloom_entry {
                bloom: bloom,
                total_put: totalput,
            });
        }
        bloomgc {
            receiver: rx,
            bloomfilter: Bloom::from_existing(
                &[0; bitmap_size],
                number_of_bits,
                number_of_hash_functions,
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
        let mut nt = tick(ndt.to_std().unwrap());
        let t = tick(Duration::from_secs(10));
        loop {
            select! {
                recv(self.receiver) -> path => {
                    match path{
                        Ok(p) => self.bloomfilter.set(&p),
                        Err(_) => {},
                    };
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

                    self.all_bloomfilter.push(bloom_entry{
                        bloom: self.bloomfilter.clone(),
                        total_put: totalp,
                    });

                    self.bloomfilter.clear();
                    config::total_put.swap(0,Ordering::SeqCst);

                    let ndt = chrono::Local.ymd(dt.year(), dt.month(), dt.day()+1).and_hms_milli(0, 0, 0, 0)-dt;
                    let mut nt = tick(ndt.to_std().unwrap());
                    self.store.append_to_all_bloom(rec);
                }
            }
        }
    }
}
