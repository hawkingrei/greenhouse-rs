pub mod store;

use crossbeam_channel::after;
use crossbeam_channel::Receiver;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct bloomgc {
    receiver: Receiver<PathBuf>,
}

impl bloomgc {
    pub fn new(rx: Receiver<PathBuf>, p: PathBuf) -> bloomgc {
        bloomgc {
            receiver: rx,
        }
    }

    pub fn serve(self) {
        let timeout = after(Duration::from_secs(1));
        loop {
            select! {
                recv(self.receiver) -> path =>  println!("elapsed: {:?}", path),
                recv(timeout) -> _ => println!("save the bloomgc"),
            }
        }
    }
}
