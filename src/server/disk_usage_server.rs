use log::info;
use rocket::config::LoggingLevel;
use rocket::config::{Config, Environment, Limits};
use std::io;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::router;
#[macro_use]
use crate::util::macros;
use crate::disk::get_disk_usage_prom;

pub struct disk_usage_server {
    metric_handle: Option<thread::JoinHandle<()>>,
    duration: Duration,
    path: PathBuf,
}

impl disk_usage_server {
    pub fn new(d: Duration, p: PathBuf) -> disk_usage_server {
        disk_usage_server {
            metric_handle: None,
            duration: d,
            path: p,
        }
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        let builder = thread::Builder::new().name(thd_name!(format!("{}", "disk-usage-service")));
        let d = self.duration.clone();
        let p = self.path.clone();
        let h = builder.spawn(move || loop {
            info!("disk metric start");
            thread::sleep(d);
            get_disk_usage_prom(p.as_path());
        })?;
        self.metric_handle = Some(h);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), io::Error> {
        info!("stop disk metric server");
        let h = match self.metric_handle.take() {
            None => return Ok(()),
            Some(h) => h,
        };
        if let Err(e) = h.join() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "join disk metric thread err",
            ));
        }
        Ok(())
    }
}
