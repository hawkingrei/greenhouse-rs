mod metric;
mod storage_handle;

use std::convert::TryInto;
use std::net;
use std::path::Path;
use std::sync::Arc;
use std::time;

use actix_http::KeepAlive;
use actix_web::{http::header::ContentEncoding, middleware::Compress, web, App, HttpServer};
use moni_middleware::Moni;
use net2::{unix::UnixTcpBuilderExt, TcpBuilder};
use storage::{DiskMetric, LazygcServer, Storage};

use crate::config::Config;
use crate::route::metric::metric;
use crate::route::storage_handle::{read, write};

#[inline]
fn unused_addr(address: String) -> net::SocketAddr {
    let addr: net::SocketAddr = address.parse().unwrap();
    let socket = TcpBuilder::new_v4().unwrap();
    socket.bind(&addr).unwrap();
    socket.reuse_address(true).unwrap();
    let tcp = socket.to_tcp_listener().unwrap();
    tcp.local_addr().unwrap()
}

pub async fn run(cfg: &Config) {
    let sys = actix_rt::System::new("greenhouse");
    let storage_config = cfg.storage.clone();
    let pathbuf = Path::new(&storage_config.cache_dir.clone()).to_path_buf();
    let ten_millis = time::Duration::from_secs(2);
    let mut metric_backend = DiskMetric::new(ten_millis, pathbuf.clone());
    let mut lazygc_backend = LazygcServer::new(pathbuf.clone(), 0.8, 0.6);
    metric_backend.start().unwrap();
    lazygc_backend.start().unwrap();
    cibo_util::metrics::monitor_threads("greenhouse")
        .unwrap_or_else(|e| crit!("failed to start monitor thread: {}", e));
    let listener = unused_addr(cfg.http_service.addr.clone());
    HttpServer::new(move || {
        App::new()
            .wrap(Moni::new())
            .wrap(Compress::new(ContentEncoding::Gzip))
            .data(Arc::new(Storage::new(storage_config.clone())))
            .service(
                web::resource("/{tail:.*}")
                    .route(web::get().to(read))
                    .route(web::put().to(write)),
            )
    })
    .workers(cfg.http_service.http_worker)
    .client_timeout(cfg.http_service.client_timeout.as_millis())
    .client_shutdown(cfg.http_service.client_shutdown.as_millis())
    .keep_alive(KeepAlive::Timeout(
        cfg.http_service.keepalive.as_secs().try_into().unwrap(),
    ))
    .bind(format!("{}", listener))
    .unwrap_or_else(|_| panic!("Can not bind to {}", &cfg.http_service.addr))
    .run();
    info!("listen to {}", &cfg.http_service.addr);

    HttpServer::new(move || App::new().route("/prometheus", web::get().to(metric)))
        .workers(1)
        .bind(&cfg.metric.address.clone())
        .unwrap_or_else(|_| panic!("Can not bind to {}", &cfg.metric.address))
        .run();
    sys.run().expect("Fail to run");
}
