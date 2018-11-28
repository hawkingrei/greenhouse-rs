use log::info;
use std::io;
use std::path::PathBuf;
use std::{thread, time};

use crate::diskgc::lazy;

pub struct LazygcServer {
    lazygc_handle: Option<thread::JoinHandle<()>>,
    path: PathBuf,
    min_percent_block_free: f64,
    stop_percent_block: f64,
}

impl LazygcServer {
    pub fn new(
        path: PathBuf,
        min_percent_block_free: f64,
        stop_percent_block: f64,
    ) -> LazygcServer {
        LazygcServer {
            lazygc_handle: None,
            path: path,
            min_percent_block_free: min_percent_block_free,
            stop_percent_block: stop_percent_block,
        }
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        let builder = thread::Builder::new().name(thd_name!("lazygc-service".to_string()));
        let gc = lazy::Lazygc::new(
            self.path.as_path(),
            self.min_percent_block_free,
            self.stop_percent_block,
        );
        let gc_millis = time::Duration::from_secs(5);
        let h = builder.spawn(move || loop {
            info!("lazy gc start");
            gc.clone().rocket();
            thread::sleep(gc_millis);
        })?;
        self.lazygc_handle = Some(h);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), io::Error> {
        info!("stop lazy gc server");
        let h = match self.lazygc_handle.take() {
            None => return Ok(()),
            Some(h) => h,
        };
        if let Err(e) = h.join() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "join lazy gc thread err",
            ));
        }
        Ok(())
    }
}
