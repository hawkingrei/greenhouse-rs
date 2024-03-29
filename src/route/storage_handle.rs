use actix_web::{web, Error, HttpRequest, HttpResponse};
use futures::StreamExt;
use storage::Storage;

pub async fn delete<'a>(req: HttpRequest, storage: web::Data<Storage>) -> HttpResponse {
    let mut url = req.uri().to_string();
    url.remove(0);
    let data = storage.get_ref().delete(url).await;
    match data {
        Ok(()) => HttpResponse::Ok().content_type("text/plain").finish(),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
pub async fn read<'a>(req: HttpRequest, storage: web::Data<Storage>) -> HttpResponse {
    let mut url = req.uri().to_string();
    url.remove(0);
    let data = storage.get_ref().read(url).await;
    match data {
        Ok(result) => HttpResponse::Ok().content_type("text/plain").body(result),
        Err(e) => {
            error!("fail to read";"err" => e.to_string(),"url" => req.uri().to_string());
            HttpResponse::NotFound().finish()
        }
    }
}

pub async fn write<'a>(
    req: HttpRequest,
    mut body: web::Payload,
    storage: web::Data<Storage>,
) -> Result<HttpResponse, Error> {
    let mut url = req.uri().to_string();
    url.remove(0);

    let mut buf = web::BytesMut::new();
    while let Some(item) = body.next().await {
        buf.extend_from_slice(&item?);
    }
    match storage.get_ref().write(buf.to_vec(), url.clone()).await {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => {
            error!("fail to writing";"url" => url,"err" => e.to_string());
            Ok(HttpResponse::BadRequest()
                .content_type("text/plain")
                .body(e.to_string()))
        }
    }
}
