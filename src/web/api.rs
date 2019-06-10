use crate::handler::service::{ControllerError, ServiceController};
use crate::messages::*;
use crate::web::models::*;
use actix::prelude::*;
use actix_web::{web, App, Error, HttpResponse, Responder};

pub fn index(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(GetOutput {
            id: item.into_inner().service,
        })
        .map_err(|e| panic!("{}", e))
        .and_then(|response| match response {
            Ok(v) => Ok(HttpResponse::Ok().body(v)),
            Err(e) => {
                warn!("{}", e);
                Ok(HttpResponse::InternalServerError().finish())
            }
        })
}

pub fn input(
    item: web::Path<ServiceRequest>,
    data: web::Json<String>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(SendStdin {
            id: item.into_inner().service,
            input: data.into_inner(),
        })
        .map_err(|e| panic!("{}", e))
        .and_then(|response| match response {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(e) => Ok(match e {
                ControllerError::InvalidInstance(_) => {
                    HttpResponse::BadRequest().body("invalid instance")
                }
                ControllerError::ServiceRunning => {
                    HttpResponse::Conflict().body("Instance not running!")
                }
                ControllerError::BrokenPipe => HttpResponse::InternalServerError().body("Broken pipe!"),
                v => {
                    warn!("Error on stdin for service: {}", v);
                    HttpResponse::InternalServerError().finish()
                }
            }),
        })
}

pub fn start(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(StartService {
            id: item.into_inner().service,
        })
        .map_err(|e| panic!("{}", e))
        .and_then(|response| match response {
            Ok(v) => Ok(HttpResponse::Ok().finish()),
            Err(e) => Ok(match e {
                ControllerError::InvalidInstance(_) => {
                    HttpResponse::BadRequest().body("invalid instance")
                }
                ControllerError::ServiceRunning => HttpResponse::Conflict().body("Already running"),
                v => {
                    warn!("Error starting service {}", v);
                    HttpResponse::InternalServerError().finish()
                }
            }),
        })
}

pub fn stop(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(StopService {
            id: item.into_inner().service,
        })
        .map_err(|e| panic!("{}", e))
        .and_then(|response| match response {
            Ok(v) => Ok(HttpResponse::Ok().finish()),
            Err(e) => Ok(match e {
                ControllerError::InvalidInstance(_) => {
                    HttpResponse::BadRequest().body("invalid instance")
                }
                ControllerError::ServiceStopped => HttpResponse::Conflict().body("Already stopped"),
                v => {
                    warn!("Error stopping service {}", v);
                    HttpResponse::InternalServerError().finish()
                }
            }),
        })
}

// pub fn login(data: web::Json<LoginUser>) -> impl Future<Item = HttpResponse, Error = Error> {
//     UserController::from_registry()
//     .send()
// }

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
