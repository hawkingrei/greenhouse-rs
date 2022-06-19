mod metric;
mod storage_handle;

use std::convert::TryInto;
use std::net;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time;
use std::time::Duration;

use actix_web::web::Data;
use actix_web::{dev::ServerHandle, rt, web, App, HttpServer};
use moni_middleware::Moni;
use net2::TcpBuilder;
use storage::{DiskMetric, LazygcServer, Storage};
use tokio::runtime;
use tokio::runtime::Runtime;

use crate::config::Config;
use crate::route::metric::metric;
use crate::route::storage_handle::{delete, read, write};

#[inline]
fn unused_addr(address: String) -> net::SocketAddr {
    let addr: net::SocketAddr = address.parse().unwrap();
    let socket = TcpBuilder::new_v4().unwrap();
    socket.bind(&addr).unwrap();
    socket.reuse_address(true).unwrap();
    let tcp = socket.to_tcp_listener().unwrap();
    tcp.local_addr().unwrap()
}

async fn run_app(tx: mpsc::Sender<ServerHandle>, cfg: &Config) -> std::io::Result<()> {
    info!("listen to {}", &cfg.http_service.addr);
    let storage_config = cfg.storage.clone();

    // srv is server controller type, `dev::Server`
    let listener = unused_addr(cfg.http_service.addr.clone());
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Moni::new())
            .app_data(Data::new(Storage::new(storage_config.clone())))
            .service(
                web::resource("/{tail:.*}")
                    .route(web::get().to(read))
                    .route(web::put().to(write))
                    .route(web::delete().to(delete)),
            )
    })
    .workers(cfg.http_service.http_worker)
    .client_request_timeout(Duration::from_millis(
        cfg.http_service.client_timeout.as_millis(),
    ))
    .client_disconnect_timeout(Duration::from_millis(
        cfg.http_service.client_shutdown.as_millis(),
    ))
    .keep_alive(Duration::from_secs(
        cfg.http_service.keepalive.as_secs().try_into().unwrap(),
    ))
    .bind(format!("{}", listener))
    .unwrap_or_else(|_| panic!("Can not bind to {}", &cfg.http_service.addr))
    .run();
    server.await
}

async fn run_metrics(metric_address: String) -> std::io::Result<()> {
    let server = HttpServer::new(move || App::new().route("/prometheus", web::get().to(metric)))
        .workers(1)
        .bind(metric_address.clone())
        .unwrap_or_else(move |_| panic!("Can not bind to {}", metric_address))
        .run();
    server.await
}

pub fn run(cfg: Config) {
    let storage_config = cfg.storage.clone();
    let pathbuf = Path::new(&storage_config.cache_dir).to_path_buf();
    let ten_millis = time::Duration::from_secs(2);
    let mut metric_backend = DiskMetric::new(ten_millis, pathbuf.clone());
    let mut lazygc_backend = LazygcServer::new(pathbuf.clone(), 0.8, 0.6);
    let metric_address = cfg.metric.address.clone();
    metric_backend.start().unwrap();
    lazygc_backend.start().unwrap();
    cibo_util::metrics::monitor_threads("greenhouse")
        .unwrap_or_else(|e| crit!("failed to start monitor thread: {}", e));
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let server_future = run_metrics(metric_address.clone());
        rt::System::with_tokio_rt(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_stack_size(1024 * 1024 * 1024)
                .build()
                .unwrap()
        })
        .block_on(server_future)
    });

    let server_future = run_app(tx, &cfg);
    rt::System::with_tokio_rt(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_name("http")
            .thread_stack_size(1024 * 1024 * 1024)
            .build()
            .unwrap()
    })
    .block_on(server_future);

    let server_handle = rx.recv().unwrap();
    rt::System::new().block_on(server_handle.stop(true));
}
