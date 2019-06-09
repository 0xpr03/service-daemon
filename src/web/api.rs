use crate::messages::*;
use crate::handler::service::ServiceController;
use crate::web::models::*;
use actix::prelude::*;
use actix_web::{web, App, Error, HttpResponse, Responder};

pub fn output(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(GetOutput {
            id: item.into_inner().service,
        })
        .map_err(Error::from)
        .and_then(|response| match response {
            Ok(v) => Ok(HttpResponse::Ok().json(v)),
            Err(e) => {
                warn!("{}", e);
                Ok(HttpResponse::InternalServerError().finish())
            }
        })
}


pub fn services() -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(GetServices {})
        .map_err(Error::from)
        // .map_err(|e|{ error!("{}", e); ()})
        .and_then(|response| match response {
            Ok(v) => Ok(HttpResponse::Ok().json(v)),
            Err(e) => {
                warn!("{}", e);
                Ok(HttpResponse::InternalServerError().finish())
            }
        })
}