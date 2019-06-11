use crate::handler::service::{ControllerError, ServiceController};
use crate::handler::user::UserService;
use crate::messages::*;
use crate::web::models::*;
use actix::prelude::*;
use actix_web::middleware::identity::Identity;
use actix_web::{web, App, Error, HttpResponse, Responder};
use futures::future::{err, ok, Either};

pub fn index(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(GetOutput {
            id: item.into_inner().service,
        })
        .map_err(Error::from)
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().body(v),
            Err(e) => {
                warn!("{}", e);
                HttpResponse::InternalServerError().finish()
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
        .map_err(Error::from)
        .map(|response| match response {
            Ok(()) => HttpResponse::Ok().finish(),
            Err(e) => match e {
                ControllerError::InvalidInstance(_) => {
                    HttpResponse::BadRequest().body("invalid instance")
                }
                ControllerError::ServiceRunning => {
                    HttpResponse::Conflict().body("Instance not running!")
                }
                ControllerError::BrokenPipe => {
                    HttpResponse::InternalServerError().body("Broken pipe!")
                }
                v => {
                    warn!("Error on stdin for service: {}", v);
                    HttpResponse::InternalServerError().finish()
                }
            },
        })
}

pub fn start(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(StartService {
            id: item.into_inner().service,
        })
        .map_err(Error::from)
        .map(|response| match response {
            Ok(()) => HttpResponse::Ok().finish(),
            Err(e) => match e {
                ControllerError::InvalidInstance(_) => {
                    HttpResponse::BadRequest().body("invalid instance")
                }
                ControllerError::ServiceRunning => HttpResponse::Conflict().body("Already running"),
                v => {
                    warn!("Error starting service {}", v);
                    HttpResponse::InternalServerError().finish()
                }
            },
        })
}

pub fn stop(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(StopService {
            id: item.into_inner().service,
        })
        .map_err(Error::from)
        .map(|response| match response {
            Ok(()) => HttpResponse::Ok().finish(),
            Err(e) => match e {
                ControllerError::InvalidInstance(_) => {
                    HttpResponse::BadRequest().body("invalid instance")
                }
                ControllerError::ServiceStopped => HttpResponse::Conflict().body("Already stopped"),
                v => {
                    warn!("Error stopping service {}", v);
                    HttpResponse::InternalServerError().finish()
                }
            },
        })
}

pub fn login(
    data: web::Json<Login>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    if let Some(session) = id.identity() {
        let data = data.into_inner();
        Either::A(
            UserService::from_registry() // LoginUser
                .send(LoginUser {
                    email: data.email,
                    password: data.password,
                    session,
                })
                .map_err(Error::from)
                .map(|resp| match resp {
                    Ok(v) => HttpResponse::Ok().json(v),
                    Err(e) => {
                        warn!("{}", e);
                        HttpResponse::InternalServerError().finish()
                    }
                }),
        )
    } else {
        debug!("No session cookies found on login request!");
        Either::B(ok(
            HttpResponse::BadRequest().body("Missing session cookies!")
        ))
    }
}

pub fn output(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(GetOutput {
            id: item.into_inner().service,
        })
        .map_err(Error::from)
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => {
                warn!("{}", e);
                HttpResponse::InternalServerError().finish()
            }
        })
}

pub fn services() -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(GetServices {})
        .map_err(Error::from)
        // .map_err(|e|{ error!("{}", e); ()})
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => {
                warn!("{}", e);
                HttpResponse::InternalServerError().finish()
            }
        })
}
