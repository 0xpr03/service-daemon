use crate::db::models::ServicePerm;
use crate::handler::error::UserError;
use crate::handler::service::ServiceController;
use crate::handler::user::UserService;
use crate::messages::*;
use crate::web::models::*;
use actix::prelude::*;
use actix_identity::*;
use actix_web::{error::ResponseError, web, Error, HttpResponse};
use futures::future::{err, ok, Either};
use nanoid;

/// Execute $cmd if $session is logged in & has $perm:ServicePerm on $service:SID
macro_rules! check_perm {
    ($session:expr,$service:expr,$perm:expr,$cmd:expr) => {
        if let Some(session) = $session {
            Either::A(
                UserService::from_registry()
                    .send(GetServicePerm {
                        service: $service,
                        session: session,
                    })
                    .map_err(Error::from)
                    .and_then(move |res| match res {
                        Ok(perms) => {
                            if perms.contains($perm) {
                                Either::A($cmd())
                            } else {
                                Either::B(Either::A(ok(
                                    HttpResponse::Unauthorized().json("no perms")
                                )))
                            }
                        }
                        Err(e) => Either::B(Either::B(ok(e.error_response()))),
                    }),
            )
        } else {
            Either::B(ok(UserError::InvalidSession.error_response()))
        }
    };
}

pub fn index(item: web::Path<ServiceRequest>) -> impl Future<Item = HttpResponse, Error = Error> {
    #[cfg(debug_assertions)]
    {
        // disable on release for now, remove eventually
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
    #[cfg(not(debug_assertions))]
    ok(HttpResponse::NotImplemented().finish())
}

pub fn state(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::STOP, move || {
        ServiceController::from_registry()
            .send(GetServiceState { id: service })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(v) => HttpResponse::Ok().json(v),
                Err(e) => e.error_response(),
            })
    })
}

pub fn input(
    item: web::Path<ServiceRequest>,
    data: web::Json<String>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::STDIN_ALL, move || {
        ServiceController::from_registry()
            .send(SendStdin {
                id: service,
                input: data.into_inner(),
            })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(()) => HttpResponse::Ok().finish(),
                Err(e) => e.error_response(),
            })
    })
}

pub fn start(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::START, move || {
        ServiceController::from_registry()
            .send(StartService { id: service })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(()) => HttpResponse::Ok().finish(),
                Err(e) => e.error_response(),
            })
    })
}

pub fn stop(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::STOP, move || {
        ServiceController::from_registry()
            .send(StopService { id: service })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(()) => HttpResponse::Ok().finish(),
                Err(e) => e.error_response(),
            })
    })
}

pub fn logout(id: Identity) -> impl Future<Item = HttpResponse, Error = Error> {
    if let Some(session) = id.identity() {
        id.forget();
        Either::A(
            UserService::from_registry()
                .send(LogoutUser { session })
                .map_err(Error::from)
                .map(|resp| match resp {
                    Ok(_) => HttpResponse::Accepted().json(true),
                    Err(e) => {
                        warn!("Logout: {}", e);
                        e.error_response()
                    }
                }),
        )
    } else {
        Either::B(ok(HttpResponse::BadRequest().json("invalid session")))
    }
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
                LoginState::LoggedIn(_) => HttpResponse::Accepted().json(v),
                LoginState::NotLoggedIn => HttpResponse::Forbidden().json(v),
                LoginState::RequiresTOTP => HttpResponse::Accepted().json(v),
                LoginState::RequiresTOTPSetup(_) => HttpResponse::Accepted().json(v),
            },
            Err(e) => {
                warn!("Login-core: {}", e);
                e.error_response()
            }
        })
}

pub fn checklogin(id: Identity) -> impl Future<Item = HttpResponse, Error = Error> {
    if let Some(session) = id.identity() {
        Either::A(
            UserService::from_registry()
                .send(CheckSession { session })
                .from_err()
                .and_then(|resp| {
                    let v = match resp {
                        Err(e) => return err(Error::from(e)),
                        Ok(v) => v,
                    };
                    ok(HttpResponse::Ok().json(v))
                }),
        )
    } else {
        Either::B(ok(HttpResponse::Ok().json(LoginState::NotLoggedIn)))
    }
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
                        LoginState::LoggedIn(_) => HttpResponse::Accepted().json(v),
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
                        LoginState::LoggedIn(_) => {
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

pub fn output(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::STDOUT, move || {
        ServiceController::from_registry()
            .send(GetOutput { id: service })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(v) => HttpResponse::Ok().json(v),
                Err(e) => e.error_response(),
            })
    })
}

pub fn services(id: Identity) -> impl Future<Item = HttpResponse, Error = Error> {
    // also checks login
    if let Some(session) = id.identity() {
        Either::A(
            ServiceController::from_registry()
                .send(GetUserServices { session })
                .map_err(Error::from)
                .map(|response| match response {
                    Ok(v) => HttpResponse::Ok().json(v),
                    Err(e) => e.error_response(),
                }),
        )
    } else {
        Either::B(ok(HttpResponse::BadRequest().json("invalid session")))
    }
}
