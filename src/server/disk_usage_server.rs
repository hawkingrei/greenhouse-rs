use log::info;
use rocket::config::LoggingLevel;
use rocket::config::{Config, Environment, Limits};
use std::io;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::disk::get_disk_usage_prom;

pub struct DiskUsageServer {
    metric_handle: Option<thread::JoinHandle<()>>,
    duration: Duration,
    path: PathBuf,
}

impl DiskUsageServer {
    pub fn new(d: Duration, p: PathBuf) -> DiskUsageServer {
        DiskUsageServer {
            metric_handle: None,
            duration: d,
            path: p,
        }
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        let builder = thread::Builder::new().name(thd_name!("disk-usage-service".to_string()));
        let d = self.duration;
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
