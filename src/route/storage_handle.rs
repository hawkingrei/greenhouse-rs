use std::sync::Arc;

use actix_web::{web, Error, HttpRequest, HttpResponse};
use futures::StreamExt;
use storage::Storage;

pub async fn delete<'a>(req: HttpRequest, storage: web::Data<Arc<Storage>>) -> HttpResponse {
    let mut url = req.uri().to_string();
    url.remove(0);
    let data = storage.delete(url).await;
    match data {
        Ok(()) => HttpResponse::Ok().content_type("text/plain").finish(),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
pub async fn read<'a>(req: HttpRequest, storage: web::Data<Arc<Storage>>) -> HttpResponse {
    let mut url = req.uri().to_string();
    url.remove(0);
    let data = storage.read(url).await;
    if let Ok(result) = data {
        HttpResponse::Ok().content_type("text/plain").body(result)
    } else {
        HttpResponse::NotFound().finish()
    }
}

pub async fn write<'a>(
    req: HttpRequest,
    mut body: web::Payload,
    storage: web::Data<Arc<Storage>>,
) -> Result<HttpResponse, Error> {
    let mut url = req.uri().to_string();
    url.remove(0);

    let mut buf = web::BytesMut::new();
    while let Some(item) = body.next().await {
        buf.extend_from_slice(&item?);
    }
    match storage.write(buf.to_vec(), url.clone()).await {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => {
            error!("fail to writing";"url" => url,"err" => e.to_string());
            Ok(HttpResponse::BadRequest()
                .content_type("text/plain")
                .body(e.to_string()))
        }
    }
}
