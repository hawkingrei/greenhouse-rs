use actix_web::{HttpRequest, HttpResponse};
use prometheus::{Encoder, TextEncoder};

pub async fn metric<'a>(_req: HttpRequest) -> HttpResponse {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    let metric_familys = prometheus::gather();
    for mf in metric_familys {
        if encoder.encode(&[mf], &mut buffer).is_err() {
            return HttpResponse::InternalServerError().body("");
        }
    }
    HttpResponse::Ok().content_type("text/plain").body(buffer)
}
