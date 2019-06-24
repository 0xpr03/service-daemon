use crate::handler::error::*;
use crate::handler::service::{ControllerError, ServiceController};
use crate::handler::user::UserService;
use crate::messages::*;
use crate::web::models::*;
use actix::prelude::*;
use actix_identity::*;
use actix_web::{error::ResponseError, web, App, Error, HttpResponse, Responder};
use futures::future::{err, ok, Either};
use nanoid;

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

pub fn logout(id: Identity) -> HttpResponse {
    trace!("Logout..");
    id.forget();
    HttpResponse::Ok().json("logout")
}

fn login_core(session: String, data: Login) -> impl Future<Item = HttpResponse, Error = Error> {
    UserService::from_registry() // LoginUser
        .send(LoginUser {
            email: data.email,
            password: data.password,
            session,
        })
        .map_err(Error::from)
        .map(|resp| match resp {
            Ok(LoginState::LoggedIn) => HttpResponse::Accepted().json("success"),
            Ok(LoginState::NotLoggedIn) => HttpResponse::Forbidden().json("invalid_login"),
            Ok(LoginState::Requires_TOTP) => HttpResponse::Ok().json("requires_totp"),
            Ok(LoginState::Requires_TOTP_Setup) => HttpResponse::Ok().json("requires_totp_setup"),
            Err(e) => {
                warn!("{}", e);
                e.error_response()
            }
        })
}

pub fn login(
    data: web::Json<Login>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let data = data.into_inner();
    if let Some(session) = id.identity() {
        Either::A(
            UserService::from_registry()
                .send(CheckSession {
                    session: session.clone(),
                })
                .from_err()
                .and_then(|resp| {
                    let val = match resp {
                        Err(e) => return Either::A(err(Error::from(e))),
                        Ok(v) => v,
                    };
                    match val {
                        LoginState::LoggedIn => {
                            Either::B(Either::A(ok(HttpResponse::BadRequest().json("logged_in"))))
                        }
                        _ => Either::B(Either::B(login_core(session, data))),
                    }
                }),
        )
    } else {
        id.remember(nanoid::generate(64));
        Either::B(login_core(id.identity().unwrap(), data))
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
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => {
                warn!("{}", e);
                HttpResponse::InternalServerError().finish()
            }
        })
}
