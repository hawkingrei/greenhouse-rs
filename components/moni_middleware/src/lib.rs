#[macro_use(slog_info)]
extern crate slog;
#[macro_use]
extern crate slog_global;
#[macro_use]
extern crate lazy_static;

mod metrics;

use std::collections::HashSet;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::http::Method;
use actix_web::{
    dev::{BodySize, MessageBody, ResponseBody, ServiceRequest, ServiceResponse},
    error::Error,
};
use bytes::Bytes;
use futures::future::{ok, Ready};
use time;

use crate::metrics::*;

pub struct Moni(Rc<Inner>);

struct Inner {
    exclude: HashSet<String>,
}

impl Moni {
    /// Create `Moni` middleware with the specified `format`.
    pub fn new() -> Moni {
        Moni(Rc::new(Inner {
            exclude: HashSet::new(),
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
        }))
    }
}

impl<S, B> Transform<S> for Moni
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<MoniLog<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = MoniMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(MoniMiddleware {
            service,
            inner: self.0.clone(),
        })
    }
}

/// Moni middleware
pub struct MoniMiddleware<S> {
    inner: Rc<Inner>,
    service: S,
}

impl<S, B> Service for MoniMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<MoniLog<B>>;
    type Error = Error;
    type Future = MoniResponse<S, B>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        TOTAL_TRANSACTION.inc();
        if self.inner.exclude.contains(req.path()) {
            MoniResponse {
                fut: self.service.call(req),
                info: None,
                time: time::now(),
                _t: PhantomData,
            }
        } else {
            let now = time::now();
            let mut info: Info = Default::default();
            info.method = req.method().clone();
            info.url_path = req.path().to_string();
            MoniResponse {
                fut: self.service.call(req),
                info: Some(info),
                time: now,
                _t: PhantomData,
            }
        }
    }
}

#[doc(hidden)]
#[pin_project::pin_project]
pub struct MoniResponse<S, B>
where
    B: MessageBody,
    S: Service,
{
    #[pin]
    fut: S::Future,
    time: time::Tm,
    info: Option<Info>,
    _t: PhantomData<(B,)>,
}

impl<S, B> Future for MoniResponse<S, B>
where
    B: MessageBody,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
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
        Poll::Ready(Ok(res.map_body(move |_, body| {
            ResponseBody::Body(MoniLog {
                body,
                size: 0,
                time,
                info,
            })
        })))
    }
}

pub struct MoniLog<B> {
    body: ResponseBody<B>,
    info: Option<Info>,
    size: usize,
    time: time::Tm,
}

impl<B> Drop for MoniLog<B> {
    fn drop(&mut self) {
        if let Some(ref info) = self.info {
            if info.method == Method::GET {
                if info.url_path.contains("/ac/") {
                    match info.status_code {
                        200 => {
                            ACTION_CACHE_HITS.inc();
                            GREENHOUSE_SIZE_HISTOGRAM.observe(self.size as f64);
                        }
                        404 => ACTION_CACHE_MISSES.inc(),
                        _ => GREENHOUSE_HTTP_ERROR.inc(),
                    }
                } else {
                    match info.status_code {
                        200 => {
                            CAS_HITS.inc();
                            GREENHOUSE_SIZE_HISTOGRAM.observe(self.size as f64);
                        }
                        404 => CAS_MISSES.inc(),
                        _ => GREENHOUSE_HTTP_ERROR.inc(),
                    }
                }
            }
            let rt = time::now() - self.time;
            let rt_fmt = (rt.num_nanoseconds().unwrap_or(0)) / 1_000_000;
            GREENHOUSE_BUSINESS_TIMING_SUM.inc_by(rt_fmt);
            GREENHOUSE_BUSINESS_TIMING_COUNT.inc();

            let url = format!("{} {}", info.method, info.url_path);
            info!(
                "{}",url,
                ;"status" =>  info.status_code.to_string() ,"size" => self.size.to_string(),
                "cost" => rt_fmt.to_string()
            );
        }
    }
}

impl<B: MessageBody> MessageBody for MoniLog<B> {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        match self.body.poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.size += chunk.len();
                Poll::Ready(Some(Ok(chunk)))
            }
            val => val,
        }
    }
}

#[derive(Debug, Default)]
pub struct Info {
    status_code: u16,
    url_path: String,
    method: Method,
}
