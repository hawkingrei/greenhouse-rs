pub mod store;

use crate::util::bloomfilter::Bloom;
use crossbeam_channel::after;
use crossbeam_channel::Receiver;
use std::path::{Path, PathBuf};
use std::time::Duration;

const items_count: usize = 500000;
const fp_p: f64 = 0.1;

pub struct bloomgc {
    receiver: Receiver<PathBuf>,
    bloomfilter: Bloom<PathBuf>,
}

impl bloomgc {
    pub fn new(rx: Receiver<PathBuf>, p: PathBuf) -> bloomgc {
        bloomgc {
            receiver: rx,
            bloomfilter: Bloom::new_for_fp_rate(items_count, fp_p),
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
                recv(timeout) -> _ => println!("save the bloomgc"),
            }
        }
    }
}
