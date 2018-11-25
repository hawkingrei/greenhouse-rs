use log::info;
use rocket::config::LoggingLevel;
use rocket::config::{Config, Environment, Limits};
use std::io;
use std::thread;

use crate::router;
#[macro_use]
use crate::util::macros;

pub struct MetricServer {
    metric_handle: Option<thread::JoinHandle<()>>,
    metrics_port: u16,
}

impl MetricServer {
    pub fn new(port: u16) -> MetricServer {
        MetricServer {
            metric_handle: None,
            metrics_port: port,
        }
    }

    pub fn start(&mut self) -> Result<(), io::Error> {
        let builder = thread::Builder::new().name(thd_name!(format!("{}", "metric-service")));
        let config = Config::build(Environment::Staging)
            .address("0.0.0.0")
            .port(self.metrics_port)
            .log_level(LoggingLevel::Off)
            .keep_alive(5)
            .finalize()
            .unwrap();
        let h = builder.spawn(move || {
            rocket::custom(config)
                .mount("/", routes![router::metrics_router::metrics])
                .launch();
        })?;
        self.metric_handle = Some(h);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), io::Error> {
        info!("stop metric server");
        let h = match self.metric_handle.take() {
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
