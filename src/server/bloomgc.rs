use crossbeam_channel::Receiver;
use log::info;
use rocket::config::LoggingLevel;
use rocket::config::{Config, Environment, Limits};
use std::io;
use std::path::PathBuf;
use std::{thread, time};

use crate::diskgc::bloom::Bloomgc;
use crate::diskgc::lazy;
use crate::router;

pub struct BloomgcServer {
    bloomgc_handle: Option<thread::JoinHandle<()>>,
    rx: Receiver<PathBuf>,
    path: PathBuf,
    days: usize,
}

impl BloomgcServer {
    pub fn new(rx: Receiver<PathBuf>, p: PathBuf, days: usize) -> BloomgcServer {
        BloomgcServer {
            bloomgc_handle: None,
            path: p,
            rx: rx,
            days: days,
        }
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        info!("new bgc  builder");
        let builder = thread::Builder::new().name(thd_name!("bloomgc-service".to_string()));
        let mut bgc = Bloomgc::new(self.rx.clone(), self.path.clone(), self.days);
        let h = builder.spawn(move || {
            info!("bgc start");
            bgc.serve();
        })?;
        self.bloomgc_handle = Some(h);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), io::Error> {
        info!("stop bloom gc server");
        let h = match self.bloomgc_handle.take() {
            None => return Ok(()),
            Some(h) => h,
        };
        if let Err(e) = h.join() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "join bloom gc thread err",
            ));
        }
        Ok(())
    }
}
