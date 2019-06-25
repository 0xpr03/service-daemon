use crate::handler::service::ServiceController;
use crate::handler::user::UserService;
use crate::messages::*;
use crate::web::models::*;
use actix::prelude::*;
use actix_identity::*;
use actix_web::{error::ResponseError, web, Error, HttpResponse};
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
            Err(e) => e.error_response(),
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
            Err(e) => e.error_response(),
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
            Err(e) => e.error_response(),
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
            Err(e) => e.error_response(),
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
            Ok(v) => match &v {
                LoginState::LoggedIn => HttpResponse::Accepted().json(v),
                LoginState::NotLoggedIn => HttpResponse::Forbidden().json(v),
                LoginState::RequiresTOTP => HttpResponse::Ok().json(v),
                LoginState::RequiresTOTPSetup(_) => HttpResponse::Ok().json(v),
            },
            Err(e) => {
                warn!("{}", e);
                e.error_response()
            }
        })
}

pub fn totp(
    data: web::Json<TOTPValue>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let data = data.into_inner();
    if let Some(session) = id.identity() {
        Either::A(
            UserService::from_registry()
                .send(LoginTOTP {
                    session: session.clone(),
                    totp: data,
                })
                .from_err()
                .and_then(|resp| {
                    let v = match resp {
                        Err(e) => return err(Error::from(e)),
                        Ok(v) => v,
                    };
                    ok(match &v {
                        LoginState::LoggedIn => HttpResponse::Accepted().json(v),
                        LoginState::NotLoggedIn => HttpResponse::Forbidden().json(v),
                        LoginState::RequiresTOTP => HttpResponse::Ok().json(v),
                        LoginState::RequiresTOTPSetup(_) => HttpResponse::Ok().json(v),
                    })
                }),
        )
    } else {
        Either::B(ok(HttpResponse::BadRequest().json("invalid session")))
    }
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
                            Either::B(Either::A(ok(HttpResponse::BadRequest().json(val))))
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
            Err(e) => e.error_response(),
        })
}

pub fn services() -> impl Future<Item = HttpResponse, Error = Error> {
    ServiceController::from_registry()
        .send(GetServices {})
        .map_err(Error::from)
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}
