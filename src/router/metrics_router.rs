use libc;
use prometheus::{Counter, Encoder, Gauge, HistogramVec, TextEncoder};
use rocket::http::ContentType;
use rocket::request::Request;
use rocket::response::{self, Responder};
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};

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
            encoder.encode(&[mf], &mut buffer);
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
