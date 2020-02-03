mod metric;
mod storage_handle;

use std::path::Path;
use std::sync::Arc;
use std::time;

use actix_web::{http::header::ContentEncoding, middleware::Compress, web, App, HttpServer};
use moni_middleware::Moni;
use storage::{start_cleaner, DiskMetric, Storage};

use crate::config::Config;
use crate::route::metric::metric;
use crate::route::storage_handle::{read, write};

pub async fn run(cfg: &Config) {
    info!("listen to {}", &cfg.addr);
    let sys = actix_rt::System::new("greenhouse");
    let storage_config = cfg.storage.clone();

    let pathbuf = Path::new(&storage_config.cache_dir.clone()).to_path_buf();
    let ten_millis = time::Duration::from_secs(2);
    let mut metric_backend = DiskMetric::new(ten_millis, pathbuf.clone());
    start_cleaner(pathbuf.clone(), 0.8, 0.6).await;
    cibo_util::metrics::monitor_threads("greenhouse")
        .unwrap_or_else(|e| crit!("failed to start monitor thread: {}", e));
    metric_backend.start().unwrap();
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
    .workers(cfg.http_worker)
    .bind(&cfg.addr.clone())
    .unwrap_or_else(|_| panic!("Can not bind to {}", &cfg.addr))
    .run();

    HttpServer::new(move || App::new().route("/prometheus", web::get().to(metric)))
        .workers(1)
        .bind(&cfg.metric.address.clone())
        .unwrap_or_else(|_| panic!("Can not bind to {}", &cfg.metric.address))
        .run();
    sys.run().expect("Fail to run");
}
