pub mod api;
pub mod websocket;
use crate::messages;
use crate::service::ServiceController;
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
                    .secure(false),
            )
            .service(web::resource("/").to_async(index))
    })
    // let ServiceController handle signals
    .disable_signals()
    .bind("127.0.0.1:59880")?
    .start())
}
