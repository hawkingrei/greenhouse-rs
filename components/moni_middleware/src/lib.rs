#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate slog_global;

use std::borrow::Cow;
use std::convert::TryFrom;
use std::collections::HashSet;
use std::{env, fmt::{self, Display as _}};
use futures_core::ready;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_utils::future::{ready, Ready};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::Method;
use actix_web::{
    body::{BodySize, MessageBody},
    error::Error,
};
 use actix_web::HttpResponse;
use regex::{Regex, RegexSet};
use log::{debug, warn};
use awc::ResponseBody;
use awc::http::header::HeaderName;
use bytes::Bytes;
use pin_project_lite::pin_project;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::metrics::*;

mod metrics;

pub struct Moni(Rc<Inner>);

struct Inner {
    exclude: HashSet<String>,
    log_target: Cow<'static, str>,
}

impl Moni {
    /// Create `Moni` middleware with the specified `format`.
    pub fn new() -> Moni {
        Moni(Rc::new(Inner {
            exclude: HashSet::new(),
            log_target: Cow::Borrowed(module_path!()),
        }))
    }

    /// Ignore and do not log access info for specified path.
    pub fn exclude<T: Into<String>>(mut self, path: T) -> Self {
        Rc::get_mut(&mut self.0)
            .unwrap()
            .exclude
            .insert(path.into());
        self
    }
}

impl Default for Moni {
    /// Create `Moni` middleware with format:
    fn default() -> Moni {
        Moni(Rc::new(Inner {
            exclude: HashSet::new(),
            log_target: Cow::Borrowed(module_path!()),
        }))
    }
}

impl<S, B> Transform<S, ServiceRequest> for Moni
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        B: MessageBody,
{
    type Response = ServiceResponse<MoniLog<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = MoniMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MoniMiddleware {
            service,
            inner: self.0.clone(),
        }))
    }
}

/// Moni middleware
pub struct MoniMiddleware<S> {
    inner: Rc<Inner>,
    service: S,
}

impl<S, B> Service<ServiceRequest> for MoniMiddleware<S>
    where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Response = ServiceResponse<MoniLog<B>>;
    type Error = Error;
    type Future = MoniResponse<S, B>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        TOTAL_TRANSACTION.inc();
        if self.inner.exclude.contains(req.path()) {
            MoniResponse {
                fut: self.service.call(req),
                info: None,
                time: OffsetDateTime::now_utc(),
                format: None,
                log_target: Cow::Borrowed(""),
                _phantom: Default::default()
            }
        } else {
            let now = OffsetDateTime::now_utc();
            let mut info: Info = Default::default();
            info.method = req.method().clone();
            info.url_path = req.path().to_string();
            MoniResponse {
                fut: self.service.call(req),
                info: Some(info),
                format: None,
                time: now,
                log_target: self.inner.log_target.clone(),
                _phantom: PhantomData,
            }
        }
    }
}

pin_project! {
    pub struct MoniResponse<S, B>
    where
        B: MessageBody,
        S: Service<ServiceRequest>,
    {
        #[pin]
        fut: S::Future,
        time: OffsetDateTime,
        format: Option<Format>,
        log_target: Cow<'static, str>,
        info: Option<Info>,
        _phantom: PhantomData<B>,
    }
}

impl<S, B> Future for MoniResponse<S, B>
    where
        B: MessageBody,
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Output = Result<ServiceResponse<MoniLog<B>>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let res = match futures::ready!(this.fut.poll(cx)) {
            Ok(res) => res,
            Err(e) => return Poll::Ready(Err(e)),
        };

        let time = *this.time;
        let mut info = this.info.take();
        if let Some(ref mut i) = info {
            i.status_code = res.status().as_u16();
        }
        let log_target = this.log_target.clone();
        Poll::Ready(Ok(res.map_body(move |_, body| {
            MoniLog {
                body,
                size: 0,
                time,
                info,
                log_target,
                format: None
            }
        })))
    }
}

pin_project! {
    pub struct MoniLog<B> {
        #[pin]
        body: B,
        format: Option<Format>,
        info: Option<Info>,
        size: usize,
        time: OffsetDateTime,
        log_target: Cow<'static, str>,
    }

    impl<B> PinnedDrop for MoniLog<B> {
        fn drop(this: Pin<&mut Self>) {
            if let Some(ref info) = this.info {
                if info.method == Method::GET {
                    GREENHOUSE_READING_COUNT.inc();
                    if info.url_path.contains("/ac/") {
                        match info.status_code {
                        200 => {
                            ACTION_CACHE_HITS.inc();
                            GREENHOUSE_SIZE_HISTOGRAM.observe(this.size as f64);
                        }
                        404 => ACTION_CACHE_MISSES.inc(),
                        _ => GREENHOUSE_HTTP_ERROR.inc(),
                    }
                    }
                } else {
                    GREENHOUSE_WRITING_COUNT.inc();
                    match info.status_code {
                        200 => {
                            CAS_HITS.inc();
                            GREENHOUSE_SIZE_HISTOGRAM.observe(this.size as f64);
                        }
                        404 => CAS_MISSES.inc(),
                        _ => GREENHOUSE_HTTP_ERROR.inc(),
                    }
                }
                let rt = OffsetDateTime::now_utc() - this.time;
                let rt_fmt = rt.subsec_nanoseconds() / 1_000_000;
                GREENHOUSE_BUSINESS_TIMING_SUM.inc_by(rt_fmt as i64);
                GREENHOUSE_BUSINESS_TIMING_COUNT.inc();

                let url = format!("{} {}", info.method, info.url_path);
                info!(
                    "{}",url,
                    ;"status" =>  info.status_code.to_string() ,"size" => this.size.to_string(),
                    "cost" => rt_fmt.to_string()
                );
            }

        }
    }
}

impl<B: MessageBody> MessageBody for MoniLog<B> {
    type Error = B::Error;

    #[inline]
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<actix_web::web::Bytes, Self::Error>>> {
        let this = self.project();

        match ready!(this.body.poll_next(cx)) {
            Some(Ok(chunk)) => {
                *this.size += chunk.len();
                Poll::Ready(Some(Ok(chunk)))
            }
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
    }
}

#[derive(Debug, Default)]
pub struct Info {
    status_code: u16,
    url_path: String,
    method: Method,
}


/// A formatting style for the `Logger` consisting of multiple concatenated `FormatText` items.
#[derive(Debug, Clone)]
struct Format(Vec<FormatText>);

impl Default for Format {
    /// Return the default formatting style for the `Logger`:
    fn default() -> Format {
        Format::new(r#"%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#)
    }
}

impl Format {
    /// Create a `Format` from a format string.
    ///
    /// Returns `None` if the format string syntax is incorrect.
    pub fn new(s: &str) -> Format {
        log::trace!("Access log format: {}", s);
        let fmt = Regex::new(r"%(\{([A-Za-z0-9\-_]+)\}([aioe]|xi)|[%atPrUsbTD]?)").unwrap();

        let mut idx = 0;
        let mut results = Vec::new();
        for cap in fmt.captures_iter(s) {
            let m = cap.get(0).unwrap();
            let pos = m.start();
            if idx != pos {
                results.push(FormatText::Str(s[idx..pos].to_owned()));
            }
            idx = m.end();

            if let Some(key) = cap.get(2) {
                results.push(match cap.get(3).unwrap().as_str() {
                    "a" => {
                        if key.as_str() == "r" {
                            FormatText::RealIpRemoteAddr
                        } else {
                            unreachable!()
                        }
                    }
                    "i" => {
                        FormatText::RequestHeader(HeaderName::try_from(key.as_str()).unwrap())
                    }
                    "o" => {
                        FormatText::ResponseHeader(HeaderName::try_from(key.as_str()).unwrap())
                    }
                    "e" => FormatText::EnvironHeader(key.as_str().to_owned()),
                    "xi" => FormatText::CustomRequest(key.as_str().to_owned(), None),
                    _ => unreachable!(),
                })
            } else {
                let m = cap.get(1).unwrap();
                results.push(match m.as_str() {
                    "%" => FormatText::Percent,
                    "a" => FormatText::RemoteAddr,
                    "t" => FormatText::RequestTime,
                    "r" => FormatText::RequestLine,
                    "s" => FormatText::ResponseStatus,
                    "b" => FormatText::ResponseSize,
                    "U" => FormatText::UrlPath,
                    "T" => FormatText::Time,
                    "D" => FormatText::TimeMillis,
                    _ => FormatText::Str(m.as_str().to_owned()),
                });
            }
        }
        if idx != s.len() {
            results.push(FormatText::Str(s[idx..].to_owned()));
        }

        Format(results)
    }
}

/// A string of text to be logged.
///
/// This is either one of the data fields supported by the `Logger`, or a custom `String`.
#[non_exhaustive]
#[derive(Debug, Clone)]
enum FormatText {
    Str(String),
    Percent,
    RequestLine,
    RequestTime,
    ResponseStatus,
    ResponseSize,
    Time,
    TimeMillis,
    RemoteAddr,
    RealIpRemoteAddr,
    UrlPath,
    RequestHeader(HeaderName),
    ResponseHeader(HeaderName),
    EnvironHeader(String),
    CustomRequest(String, Option<CustomRequestFn>),
}

#[derive(Clone)]
struct CustomRequestFn {
    inner_fn: Rc<dyn Fn(&ServiceRequest) -> String>,
}

impl CustomRequestFn {
    fn call(&self, req: &ServiceRequest) -> String {
        (self.inner_fn)(req)
    }
}

impl fmt::Debug for CustomRequestFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("custom_request_fn")
    }
}

impl FormatText {
    fn render(
        &self,
        fmt: &mut fmt::Formatter<'_>,
        size: usize,
        entry_time: OffsetDateTime,
    ) -> Result<(), fmt::Error> {
        match self {
            FormatText::Str(ref string) => fmt.write_str(string),
            FormatText::Percent => "%".fmt(fmt),
            FormatText::ResponseSize => size.fmt(fmt),
            FormatText::Time => {
                let rt = OffsetDateTime::now_utc() - entry_time;
                let rt = rt.as_seconds_f64();
                fmt.write_fmt(format_args!("{:.6}", rt))
            }
            FormatText::TimeMillis => {
                let rt = OffsetDateTime::now_utc() - entry_time;
                let rt = (rt.whole_nanoseconds() as f64) / 1_000_000.0;
                fmt.write_fmt(format_args!("{:.6}", rt))
            }
            FormatText::EnvironHeader(ref name) => {
                if let Ok(val) = std::env::var(name) {
                    fmt.write_fmt(format_args!("{}", val))
                } else {
                    "-".fmt(fmt)
                }
            }
            _ => Ok(()),
        }
    }

    fn render_response<B>(&mut self, res: &HttpResponse<B>) {
        match self {
            FormatText::ResponseStatus => {
                *self = FormatText::Str(format!("{}", res.status().as_u16()))
            }
            FormatText::ResponseHeader(ref name) => {
                let s = if let Some(val) = res.headers().get(name) {
                    if let Ok(s) = val.to_str() {
                        s
                    } else {
                        "-"
                    }
                } else {
                    "-"
                };
                *self = FormatText::Str(s.to_string())
            }
            _ => {}
        }
    }

    fn render_request(&mut self, now: OffsetDateTime, req: &ServiceRequest) {
        match self {
            FormatText::RequestLine => {
                *self = if req.query_string().is_empty() {
                    FormatText::Str(format!(
                        "{} {} {:?}",
                        req.method(),
                        req.path(),
                        req.version()
                    ))
                } else {
                    FormatText::Str(format!(
                        "{} {}?{} {:?}",
                        req.method(),
                        req.path(),
                        req.query_string(),
                        req.version()
                    ))
                };
            }
            FormatText::UrlPath => *self = FormatText::Str(req.path().to_string()),
            FormatText::RequestTime => *self = FormatText::Str(now.format(&Rfc3339).unwrap()),
            FormatText::RequestHeader(ref name) => {
                let s = if let Some(val) = req.headers().get(name) {
                    if let Ok(s) = val.to_str() {
                        s
                    } else {
                        "-"
                    }
                } else {
                    "-"
                };
                *self = FormatText::Str(s.to_string());
            }
            FormatText::RemoteAddr => {
                let s = if let Some(peer) = req.connection_info().peer_addr() {
                    FormatText::Str((*peer).to_string())
                } else {
                    FormatText::Str("-".to_string())
                };
                *self = s;
            }
            FormatText::RealIpRemoteAddr => {
                let s = if let Some(remote) = req.connection_info().realip_remote_addr() {
                    FormatText::Str(remote.to_string())
                } else {
                    FormatText::Str("-".to_string())
                };
                *self = s;
            }
            FormatText::CustomRequest(_, request_fn) => {
                let s = match request_fn {
                    Some(f) => FormatText::Str(f.call(req)),
                    None => FormatText::Str("-".to_owned()),
                };

                *self = s;
            }
            _ => {}
        }
    }
}

/// Converter to get a String from something that writes to a Formatter.
pub(crate) struct FormatDisplay<'a>(
    &'a dyn Fn(&mut fmt::Formatter<'_>) -> Result<(), fmt::Error>,
);

impl<'a> fmt::Display for FormatDisplay<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        (self.0)(fmt)
    }
}
