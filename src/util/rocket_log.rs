use std::sync::Arc;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::request;
use rocket::{Data, Request, Response, Rocket, State};
use slog::Logger;

/// Newtype struct wrapper around the passed-in slog::Logger
#[derive(Debug, Clone)]
pub struct SyncLogger(Arc<Logger>);

/// Fairing used to provide a rocket.rs application with a slog::Logger
#[derive(Debug, Clone)]
pub struct SlogFairing(SyncLogger);

impl SlogFairing {
    /// Create a new SlogFairing using the slog::Logger
    pub fn new(root_logger: Logger) -> SlogFairing {
        SlogFairing(SyncLogger(Arc::new(root_logger)))
    }
}

impl SyncLogger {
    pub fn get(&self) -> &Logger {
        &*self.0
    }
}

impl std::ops::Deref for SyncLogger {
    type Target = Arc<Logger>;

    fn deref(&self) -> &Arc<Logger> {
        &self.0
    }
}

impl<'a, 'r> request::FromRequest<'a, 'r> for SyncLogger {
    type Error = ();

    fn from_request(req: &'a request::Request<'r>) -> request::Outcome<SyncLogger, ()> {
        let sync_logger = req.guard::<State<SyncLogger>>()?;
        rocket::Outcome::Success(sync_logger.clone())
    }
}

impl Fairing for SlogFairing {
    fn info(&self) -> Info {
        Info {
            name: "Slog Fairing",
            kind: Kind::Attach | Kind::Launch | Kind::Request | Kind::Response,
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        {
            let config = rocket.config();
            slog_info!(&self.0, "config"; "key" => "environment", "value" => ?config.environment);
            slog_info!(&self.0, "config"; "key" => "address", "value" => %config.address);
            slog_info!(&self.0, "config"; "key" => "port", "value" => %config.port);
            slog_info!(&self.0, "config"; "key" => "workers", "value" => %config.workers);
            slog_info!(&self.0, "config"; "key" => "log_level", "value" => ?config.log_level);
            // not great, could there be a way to enumerate limits like we do for extras?
            if let Some(forms) = config.limits.get("forms") {
                slog_info!(&self.0, "config"; "key" => "forms limit", "value" => ?forms);
            }
            if let Some(json) = config.limits.get("json") {
                slog_info!(&self.0, "config"; "key" => "json limit", "value" => ?json);
            }
            if let Some(msgpack) = config.limits.get("msgpack") {
                slog_info!(&self.0, "config"; "key" => "msgpack limit", "value" => ?msgpack);
            }
            for (key, val) in &config.extras {
                slog_info!(&self.0, "config"; "key" => &key, "value" => ?val);
            }
        }
        // add managed logger so the user can use it in guards
        Ok(rocket.manage(self.0.clone()))
    }

    fn on_launch(&self, rocket: &Rocket) {
        for route in rocket.routes() {
            if route.rank < 0 {
                slog_info!(&self.0, "route"; "base" => %route.base(), "path" => %route.uri, "method" => %route.method);
            } else {
                slog_info!(&self.0, "route"; "base" => %route.base(), "path" => %route.uri, "rank" => %route.rank);
            }
        }
        // can't seem to get the list of Catchers?

        let config = rocket.config();
        let addr = format!("http://{}:{}", &config.address, &config.port);
        slog_info!(&self.0, "listening"; "address" => %addr);
    }

    fn on_request(&self, request: &mut Request, _: &Data) {
        slog_info!(self.0, "request"; "method" => ?request.method(), "uri" => ?request.uri().to_string());
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        let status = response.status();
        let status = format!("{} {}", status.code, status.reason);
        if let Some(ref route) = request.route() {
            slog_info!(&self.0, "response"; "route" => %route, "status" => %status);
        } else {
            slog_info!(&self.0, "response"; "status" => %status);
        }
    }
}
