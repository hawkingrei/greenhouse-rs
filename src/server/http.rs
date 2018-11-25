use crossbeam_channel::Sender;
use log::info;
use rocket::config::LoggingLevel;
use rocket::config::{Config, Environment, Limits};
use std::io;
use std::path::PathBuf;
use std::thread;

use crate::config::CachePath;
use crate::router;

pub struct HttpServe {
    http_handle: Option<thread::JoinHandle<()>>,
    http_addr: String,
    http_port: u16,
    path: String,
    tx: Sender<PathBuf>,
}

impl HttpServe {
    pub fn new(addr: String, port: u16, path: String, tx: Sender<PathBuf>) -> HttpServe {
        HttpServe {
            http_handle: None,
            http_addr: addr,
            http_port: port,
            path: path,
            tx: tx,
        }
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        let builder = thread::Builder::new().name(thd_name!(format!("{}", "metric-service")));
        let config = Config::build(Environment::Staging)
            .address(self.http_addr.clone())
            .port(self.http_port)
            .keep_alive(180)
            .limits(Limits::new().limit("forms", 1024 * 1024 * 512))
            .log_level(LoggingLevel::Off)
            .finalize()
            .unwrap();
        let server = rocket::custom(config)
            .manage(CachePath(self.path.clone()))
            .manage(self.tx.clone())
            .mount("/", routes![router::upload, router::get, router::head]);
        let h = builder.spawn(move || {
            server.launch();
        })?;
        self.http_handle = Some(h);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), io::Error> {
        info!("stop metric server");
        let h = match self.http_handle.take() {
            None => return Ok(()),
            Some(h) => h,
        };
        if let Err(e) = h.join() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "join metric thread err",
            ));
        }
        Ok(())
    }
}
