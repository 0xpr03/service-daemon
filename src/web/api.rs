use crate::db::models::ServicePerm;
use crate::handler::error::UserError;
use crate::handler::service::ServiceController;
use crate::handler::user::UserService;
use crate::messages::*;
use crate::web::models::*;
use actix::prelude::*;
use actix_files as fs;
use actix_identity::*;
use actix_web::{error::ResponseError, web, Error, HttpRequest, HttpResponse};
use futures::future::{err, ok, Either};
use nanoid;

/// Returns session, otherwise returns with InvalidSession http response
macro_rules! get_session {
    ($session:expr) => {
        match $session.identity() {
            Some(v) => v,
            None => return Either::B(ok(UserError::InvalidSession.error_response())),
        }
    };
}

macro_rules! check_admin {
    ($session:expr,$cmd:expr) => {
        if let Some(session) = $session {
            Either::A(
                UserService::from_registry()
                    .send(GetAdminPerm { session: session })
                    .map_err(Error::from)
                    .and_then(move |res| match res {
                        Ok(admin) => {
                            if admin {
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
                        Ok((uid, perms)) => {
                            if perms.contains($perm) {
                                Either::A($cmd(uid))
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

pub fn change_totp(
    item: web::Path<UserRequest>,
    data: web::Json<ResetTOTP>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let identity = get_session!(id);
    Either::A(
        UserService::from_registry()
            .send(ResetUserTOTP {
                invoker: identity,
                data: data.into_inner(),
                id: item.user,
            })
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(_) => HttpResponse::NoContent().finish(),
                Err(e) => e.error_response(),
            }),
    )
}

pub fn change_password(
    item: web::Path<UserRequest>,
    data: web::Json<SetPassword>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let identity = get_session!(id);
    Either::A(
        UserService::from_registry()
            .send(SetUserPassword {
                data: data.into_inner(),
                invoker: identity,
                id: item.user,
            })
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(_) => HttpResponse::NoContent().finish(),
                Err(e) => e.error_response(),
            }),
    )
}

pub fn fallback(_: HttpRequest) -> actix_web::Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/index.html")?)
}

pub fn user_list(id: Identity) -> impl Future<Item = HttpResponse, Error = Error> {
    check_admin!(id.identity(), move || {
        UserService::from_registry()
            .send(unchecked::GetAllUsers {})
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(v) => HttpResponse::Ok().json(v),
                Err(e) => e.error_response(),
            })
    })
}

pub fn get_user_info(
    item: web::Path<UserRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    check_admin!(id.identity(), move || {
        UserService::from_registry()
            .send(unchecked::GetUserInfo { user: item.user })
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(v) => HttpResponse::Ok().json(v),
                Err(e) => e.error_response(),
            })
    })
}

pub fn set_user_info(
    item: web::Path<UserRequest>,
    data: web::Json<UserMinData>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let identity = get_session!(id);
    let data = data.into_inner();
    Either::A(
        UserService::from_registry()
            .send(SetUserInfo {
                data,
                user: item.user,
                invoker: identity,
            })
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(_) => HttpResponse::NoContent().finish(),
                Err(e) => e.error_response(),
            }),
    )
}

pub fn delete_user(
    data: web::Path<UserRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    // verifies user permissions
    id.identity().map_or(
        Either::B(ok(UserError::InvalidSession.error_response())),
        |session| {
            Either::A(
                UserService::from_registry()
                    .send(DeleteUser {
                        user: data.into_inner().user,
                        invoker: session,
                    })
                    .map_err(Error::from)
                    .map(move |res| match res {
                        Ok(_) => HttpResponse::NoContent().finish(),
                        Err(e) => e.error_response(),
                    }),
            )
        },
    )
}

pub fn create_user(
    data: web::Json<NewUser>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    // verifies user permissions
    id.identity().map_or(
        Either::B(ok(UserError::InvalidSession.error_response())),
        |session| {
            Either::A(
                UserService::from_registry()
                    .send(CreateUser {
                        user: data.into_inner(),
                        invoker: session,
                    })
                    .map_err(Error::from)
                    .map(move |res| match res {
                        Ok(state) => HttpResponse::Ok().json(state),
                        Err(e) => e.error_response(),
                    }),
            )
        },
    )
}

pub fn set_service_permission(
    item: web::Path<PermRequest>,
    data: web::Json<ServicePermWrap>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    trace!("Setting service permission {:?}", data);
    check_admin!(id.identity(), move || {
        UserService::from_registry()
            .send(unchecked::SetServicePermUser {
                service: item.service,
                user: item.user,
                perm: ServicePerm::from_bits_truncate(data.into_inner().perms),
            })
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(_) => HttpResponse::NoContent().finish(),
                Err(e) => e.error_response(),
            })
    })
}

pub fn get_service_permission(
    item: web::Path<PermRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    check_admin!(id.identity(), move || {
        UserService::from_registry()
            .send(unchecked::GetServicePermUser {
                service: item.service,
                user: item.user,
            })
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(perms) => HttpResponse::Ok().json(ServicePermWrap {
                    perms: perms.bits(),
                }),
                Err(e) => e.error_response(),
            })
    })
}

pub fn all_user_services(
    item: web::Path<UserRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    check_admin!(id.identity(), move || {
        ServiceController::from_registry()
            .send(unchecked::GetUserServicePermsAll { user: item.user })
            .map_err(Error::from)
            .map(move |res| match res {
                Ok(v) => HttpResponse::Ok().json(v),
                Err(e) => e.error_response(),
            })
    })
}

/// Return service permissions of current session
pub fn session_service_perm(
    id: Identity,
    item: web::Path<ServiceRequest>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    if let Some(session) = id.identity() {
        Either::A(
            UserService::from_registry()
                .send(GetServicePerm {
                    session,
                    service: item.service,
                })
                .map_err(Error::from)
                .map(|response| match response {
                    Ok((_, v)) => HttpResponse::Ok().json(ServicePermWrap { perms: v.bits() }),
                    Err(e) => e.error_response(),
                }),
        )
    } else {
        Either::B(ok(UserError::InvalidSession.error_response()))
    }
}

/// Return management permissons of current session
pub fn session_permissions(id: Identity) -> impl Future<Item = HttpResponse, Error = Error> {
    if let Some(session) = id.identity() {
        Either::A(
            UserService::from_registry()
                .send(GetAdminPerm { session })
                .map_err(Error::from)
                .map(|response| match response {
                    Ok(v) => HttpResponse::Ok().json(v),
                    Err(e) => e.error_response(),
                }),
        )
    } else {
        Either::B(ok(UserError::InvalidSession.error_response()))
    }
}

pub fn log_latest(
    item: web::Path<LogLatestRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    dbg!(&item);
    let item = item.into_inner();
    check_perm!(id.identity(), item.service, ServicePerm::LOG, move |_| {
        ServiceController::from_registry()
            .send(unchecked::GetLogLatest {
                id: item.service,
                amount: item.amount,
            })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(v) => HttpResponse::Ok().json(v),
                Err(e) => e.error_response(),
            })
    })
}

// TODO: rewrite to also use service macro
// currently using manual perm fetching for perms.is_empty()
pub fn state(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    if let Some(session) = id.identity() {
        Either::A(
            UserService::from_registry()
                .send(GetServicePerm {
                    service,
                    session: session,
                })
                .map_err(Error::from)
                .and_then(move |res| match res {
                    Ok((_, perms)) => {
                        if perms.is_empty() {
                            Either::B(Either::A(ok(HttpResponse::Unauthorized().json("no perms"))))
                        } else {
                            Either::A(
                                ServiceController::from_registry()
                                    .send(unchecked::GetServiceState { id: service })
                                    .map_err(Error::from)
                                    .map(|response| match response {
                                        Ok(v) => HttpResponse::Ok().json(v),
                                        Err(e) => e.error_response(),
                                    }),
                            )
                        }
                    }
                    Err(e) => Either::B(Either::B(ok(e.error_response()))),
                }),
        )
    } else {
        Either::B(ok(UserError::InvalidSession.error_response()))
    }
}

pub fn input(
    item: web::Path<ServiceRequest>,
    data: web::Json<String>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::STDIN_ALL, move |uid| {
        ServiceController::from_registry()
            .send(unchecked::SendStdin {
                id: service,
                input: data.into_inner(),
                user: Some(uid),
            })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(()) => HttpResponse::NoContent().finish(),
                Err(e) => e.error_response(),
            })
    })
}

pub fn start(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::START, move |uid| {
        ServiceController::from_registry()
            .send(unchecked::StartService {
                id: service,
                user: Some(uid),
            })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(()) => HttpResponse::NoContent().finish(),
                Err(e) => e.error_response(),
            })
    })
}

pub fn kill(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::KILL, move |uid| {
        ServiceController::from_registry()
            .send(unchecked::KillService {
                id: service,
                user: Some(uid),
            })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(()) => HttpResponse::NoContent().finish(),
                Err(e) => e.error_response(),
            })
    })
}

pub fn stop(
    item: web::Path<ServiceRequest>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let service = item.into_inner().service;
    check_perm!(id.identity(), service, ServicePerm::STOP, move |uid| {
        ServiceController::from_registry()
            .send(unchecked::StopService {
                id: service,
                user: Some(uid),
            })
            .map_err(Error::from)
            .map(|response| match response {
                Ok(()) => HttpResponse::NoContent().finish(),
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
                    Err(e) => e.error_response(),
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
            Err(e) => e.error_response(),
        })
}

pub fn checklogin(id: Identity) -> impl Future<Item = HttpResponse, Error = Error> {
    if let Some(session) = id.identity() {
        Either::A(
            UserService::from_registry()
                .send(CheckSession { session })
                .from_err()
                .map(|resp| match resp {
                    Ok(v) => HttpResponse::Ok().json(v),
                    Err(e) => e.error_response(),
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
    check_perm!(id.identity(), service, ServicePerm::OUTPUT, move |_| {
        ServiceController::from_registry()
            .send(unchecked::GetOutput { id: service })
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
                .send(GetSessionServices { session })
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
