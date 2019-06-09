pub mod api;

pub mod models;
pub mod websocket;

use crate::messages;
use crate::handler::service::ServiceController;

use actix::prelude::*;
use actix_session::{CookieSession, Session};
use actix_web::dev::Server;
use actix_web::{web, App, Error, HttpResponse, HttpServer};

fn index() -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(messages::GetOutput { id: 0 })
        .map_err(|e| panic!("{}", e))
        .and_then(|response| match response {
            Ok(v) => Ok(HttpResponse::Ok().body(v)),
            Err(e) => {
                warn!("{}", e);
                Ok(HttpResponse::InternalServerError().finish())
            }
        })
}

pub fn start() -> std::io::Result<Server> {
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(
                CookieSession::signed(&[0; 32]) // <- create cookie based session middleware
                    .secure(false)
                    .http_only(false),
            )
            .data(web::JsonConfig::default().limit(4096))
            .service(
                web::resource("/api/service/{service}/output")
                    .route(web::get().to_async(api::output)),
            )
            .service(web::resource("/api/service").route(web::get().to_async(api::services)))
    })
    // let ServiceController handle signals
    // .disable_signals()
    .bind("127.0.0.1:59880")?
    .start())
}
