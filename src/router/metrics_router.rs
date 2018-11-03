use prometheus::{Encoder, TextEncoder};
use rocket::http::ContentType;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use std::io;

#[derive(Debug)]
pub struct MetricsHandle();

impl MetricsHandle {
    pub fn new() -> io::Result<MetricsHandle> {
        Ok(MetricsHandle {})
    }
}

impl<'r> Responder<'r> for MetricsHandle {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        let encoder = TextEncoder::new();
        let mut buffer = vec![];
        let metric_familys = prometheus::gather();
        for mf in metric_familys {
            match encoder.encode(&[mf], &mut buffer) {
                Ok(_) => {}
                Err(_) => return Err(Status::InternalServerError),
            }
        }
        let mut response = buffer.respond_to(req)?;
        response.set_header(ContentType::Plain);
        Ok(response)
    }
}

#[get("/prometheus")]
pub fn metrics() -> Option<MetricsHandle> {
    MetricsHandle::new().ok()
}
