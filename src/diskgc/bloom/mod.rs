pub mod spb;
pub mod store;

use chrono::offset::Local;
use crossbeam_channel::after;
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

pub struct bloomgc {
    receiver: Receiver<PathBuf>,
    bloomfilter: Bloom<PathBuf>,
    all_bloomfilter: Vec<Bloom<PathBuf>>,
    store: gc_store,
}

impl bloomgc {
    pub fn new(rx: Receiver<PathBuf>, p: PathBuf) -> bloomgc {
        bloomgc {
            receiver: rx,
            bloomfilter: Bloom::from_existing(
                &[0; bitmap_size],
                number_of_bits,
                number_of_hash_functions,
                [(2749812374, 12341234), (574893759834, 1298374918234)],
            ),
            store: new_gc_store(p),
            all_bloomfilter: Vec::new(),
        }
    }

    pub fn serve(&mut self) {
        let timeout = after(Duration::from_secs(1));
        loop {
            select! {
                recv(self.receiver) -> path => {
                    match path{
                        Ok(p) => self.bloomfilter.set(&p),
                        Err(_) => {},
                    };
                },
                recv(timeout) -> _ => {
                    let mut rec = Record::new();
                    let mut now:Timestamp = Timestamp::new();
                    now.set_seconds(Local::now().timestamp());
                    rec.set_time(now);
                    rec.set_data(self.bloomfilter.bitmap());
                    rec.set_totalPut(config::total_put.load(Ordering::SeqCst) as u64);
                    let result = rec.write_to_bytes().unwrap();

                },
            }
        }
    }
}
