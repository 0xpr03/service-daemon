use crate::db::models::ServicePerm;
use crate::handler::error::UserError;
use crate::handler::service::ServiceController;
use crate::handler::user::UserService;
use crate::messages::*;
use crate::web::models::*;
use actix::prelude::*;
use actix_files as fs;
use actix_identity::*;
use actix_web::{error::ResponseError, web, HttpMessage, HttpRequest, HttpResponse};
use log::*;
use nanoid::nanoid;

/// Returns session, otherwise returns with InvalidSession http response
// macro_rules! get_session_async {
//     ($session:expr) => {
//         match $session.id() {
//             Some(v) => v,
//             None => return Ok(UserError::InvalidSession.error_response()),
//         }
//     };
// }

#[derive(thiserror::Error, Debug)]
pub enum APIError {
    #[error("User Error")]
    UserError(#[from] UserError),
    #[error("Internal error performing message passing: {0}")]
    InternalMailBox(#[from] MailboxError),
    #[error("Internal error accessing identity ID: {0}")]
    InternalIdentity(#[from] anyhow::Error),
}

type Result<E = HttpResponse> = std::result::Result<E, APIError>;

impl ResponseError for APIError {
    fn error_response(&self) -> HttpResponse {
        match self {
            APIError::UserError(e) => e.error_response(),
            v => {
                error!("{}", v);
                HttpResponse::InternalServerError().json("Internal Server Error, Please try later")
            }
        }
    }
}

/// Assert admin privileges for valid session, otherwise returns
macro_rules! assert_admin {
    ($session:expr) => {
        let ret = UserService::from_registry()
            .send(GetAdminPerm {
                session: $session.id().map_err(|_| UserError::InvalidSession)?,
            })
            .await?;
        match ret {
            Ok(admin) => {
                if !admin {
                    // only catch non-admin
                    return Ok(HttpResponse::Unauthorized().json("no perms"));
                }
            }
            Err(e) => return Ok(e.error_response()),
        }
    };
}

/// Continue if $session is logged in & has $perm:ServicePerm on $service:SID, returns on success, otherwise performs early-return
macro_rules! assert_perm {
    ($session:expr,$service:expr,$perm:expr) => {
        /*let ret = UserService::from_registry()
        .send(GetServicePerm {
            service: $service,
            session: $session.id().map_err(|_|UserError::InvalidSession)?,
        })
        .await?;*/
        match UserService::from_registry()
            .send(GetServicePerm {
                service: $service,
                session: $session.id().map_err(|_| UserError::InvalidSession)?,
            })
            .await?
        {
            Ok((uid, perms)) => {
                if perms.contains($perm) {
                    uid
                } else {
                    return Ok(HttpResponse::Unauthorized().json("no perms"));
                }
            }
            Err(e) => return Ok(e.error_response()),
        }
    };
}

pub async fn change_totp(
    item: web::Path<UserRequest>,
    data: web::Json<ResetTOTP>,
    identity: Identity,
) -> Result {
    let session = identity.id()?;
    UserService::from_registry()
        .send(ResetUserTOTP {
            invoker: session,
            data: data.into_inner(),
            id: item.user,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn change_password(
    item: web::Path<UserRequest>,
    data: web::Json<SetPassword>,
    identity: Identity,
) -> Result {
    let session = identity.id()?;
    UserService::from_registry()
        .send(SetUserPassword {
            data: data.into_inner(),
            invoker: session,
            id: item.user,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn fallback(_: HttpRequest) -> actix_web::Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/index.html")?)
}

pub async fn user_list(identity: Identity) -> Result {
    assert_admin!(identity);
    UserService::from_registry()
        .send(unchecked::GetAllUsers {})
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

pub async fn get_user_info(item: web::Path<UserRequest>, identity: Identity) -> Result {
    assert_admin!(identity);
    UserService::from_registry()
        .send(unchecked::GetUserInfo { user: item.user })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

pub async fn set_user_info(
    item: web::Path<UserRequest>,
    data: web::Json<UserMinData>,
    identity: Identity,
) -> Result {
    let session = identity.id()?;
    let data = data.into_inner();
    UserService::from_registry()
        .send(SetUserInfo {
            data,
            user: item.user,
            invoker: session,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn delete_user(data: web::Path<UserRequest>, identity: Identity) -> Result {
    // verifies user permissions
    let session = identity.id()?;
    UserService::from_registry()
        .send(DeleteUser {
            user: data.into_inner().user,
            invoker: session,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn create_user(data: web::Json<NewUser>, identity: Identity) -> Result {
    // verifies user permissions
    let session = identity.id()?;
    UserService::from_registry()
        .send(CreateUser {
            user: data.into_inner(),
            invoker: session,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(state) => HttpResponse::Ok().json(state),
            Err(e) => e.error_response(),
        })
}

pub async fn set_service_permission(
    item: web::Path<PermRequest>,
    data: web::Json<ServicePermWrap>,
    identity: Identity,
) -> Result {
    trace!("Setting service permission {:?}", data);
    assert_admin!(identity);
    UserService::from_registry()
        .send(unchecked::SetServicePermUser {
            service: item.service,
            user: item.user,
            perm: ServicePerm::from_bits_truncate(data.into_inner().perms),
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn get_service_permission(item: web::Path<PermRequest>, identity: Identity) -> Result {
    assert_admin!(identity);
    UserService::from_registry()
        .send(unchecked::GetServicePermUser {
            service: item.service,
            user: item.user,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(perms) => HttpResponse::Ok().json(ServicePermWrap {
                perms: perms.bits(),
            }),
            Err(e) => e.error_response(),
        })
}

pub async fn all_user_services(item: web::Path<UserRequest>, identity: Identity) -> Result {
    assert_admin!(identity);
    ServiceController::from_registry()
        .send(unchecked::GetUserServicePermsAll { user: item.user })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(move |res| match res {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

/// Return service permissions of current session
pub async fn session_service_perm(identity: Identity, item: web::Path<ServiceRequest>) -> Result {
    let session = identity.id()?;
    UserService::from_registry()
        .send(GetServicePerm {
            session,
            service: item.service,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok((_, v)) => HttpResponse::Ok().json(ServicePermWrap { perms: v.bits() }),
            Err(e) => e.error_response(),
        })
}

/// Return management permissons of current session
pub async fn session_permissions(identity: Identity) -> Result {
    let session = identity.id()?;
    UserService::from_registry()
        .send(GetAdminPerm { session })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

pub async fn log_latest(item: web::Path<LogLatestRequest>, identity: Identity) -> Result {
    let item = item.into_inner();
    assert_perm!(identity, item.service, ServicePerm::LOG);
    ServiceController::from_registry()
        .send(unchecked::GetLogLatest {
            id: item.service,
            amount: item.amount,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

pub async fn log_details(item: web::Path<LogRequest>, identity: Identity) -> Result {
    let item = item.into_inner();
    assert_perm!(identity, item.service, ServicePerm::LOG);
    ServiceController::from_registry()
        .send(unchecked::GetLogDetails {
            id: item.service,
            log_id: item.log_id,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

pub async fn log_console(item: web::Path<LogRequest>, identity: Identity) -> Result {
    let item = item.into_inner();
    // custom assert_perm due to output & log permissions
    let session = identity.id()?;
    let ret = UserService::from_registry()
        .send(GetServicePerm {
            service: item.service,
            session: session,
        })
        .await?;
    match ret {
        Ok((_, perms)) => {
            if !(perms.contains(ServicePerm::OUTPUT) && perms.contains(ServicePerm::LOG)) {
                return Ok(HttpResponse::Unauthorized().json("no perms"));
            }
        }
        Err(e) => return Ok(e.error_response()),
    }
    ServiceController::from_registry()
        .send(unchecked::GetLogConsole {
            id: item.service,
            log_id: item.log_id,
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

// TODO: rewrite to also use service macro
// currently using manual perm fetching for perms.is_empty()
pub async fn state(item: web::Path<ServiceRequest>, identity: Identity) -> Result {
    let service = item.into_inner().service;
    let session = identity.id()?;
    let res = UserService::from_registry()
        .send(GetServicePerm { service, session })
        .await?;
    match res {
        Ok((_, perms)) => {
            if perms.is_empty() {
                Ok(HttpResponse::Unauthorized().json("no perms"))
            } else {
                ServiceController::from_registry()
                    .send(unchecked::GetServiceState { id: service })
                    .await
                    .map_err(|e| UserError::from(e).into())
                    .map(|response| match response {
                        Ok(v) => HttpResponse::Ok().json(v),
                        Err(e) => e.error_response(),
                    })
            }
        }
        Err(e) => Ok(e.error_response()),
    }
}

pub async fn input(
    item: web::Path<ServiceRequest>,
    data: web::Json<String>,
    identity: Identity,
) -> Result {
    let service = item.into_inner().service;
    let uid = assert_perm!(identity, service, ServicePerm::STDIN_ALL);
    ServiceController::from_registry()
        .send(unchecked::SendStdin {
            id: service,
            input: data.into_inner(),
            user: Some(uid),
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(()) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn start(item: web::Path<ServiceRequest>, identity: Identity) -> Result {
    let service = item.into_inner().service;
    let uid = assert_perm!(identity, service, ServicePerm::START);
    ServiceController::from_registry()
        .send(unchecked::StartService {
            id: service,
            user: Some(uid),
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(()) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn kill(item: web::Path<ServiceRequest>, identity: Identity) -> Result {
    let service = item.into_inner().service;
    let uid = assert_perm!(identity, service, ServicePerm::KILL);
    ServiceController::from_registry()
        .send(unchecked::KillService {
            id: service,
            user: Some(uid),
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(()) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn stop(item: web::Path<ServiceRequest>, identity: Identity) -> Result {
    let service = item.into_inner().service;
    let uid = assert_perm!(identity, service, ServicePerm::STOP);
    ServiceController::from_registry()
        .send(unchecked::StopService {
            id: service,
            user: Some(uid),
        })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(()) => HttpResponse::NoContent().finish(),
            Err(e) => e.error_response(),
        })
}

pub async fn logout(identity: Identity) -> Result {
    let session = identity.id()?;
    identity.logout();
    UserService::from_registry()
        .send(LogoutUser { session })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|resp| match resp {
            Ok(_) => HttpResponse::Accepted().json(true),
            Err(e) => e.error_response(),
        })
}

async fn login_core(session: String, data: Login) -> Result {
    UserService::from_registry() // LoginUser
        .send(LoginUser {
            email: data.email,
            password: data.password,
            session,
        })
        .await
        .map_err(|e| UserError::from(e).into())
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

pub async fn checklogin(identity: Option<Identity>) -> Result {
    if let Some(identity) = identity {
        let session = identity.id()?;
        UserService::from_registry()
            .send(CheckSession { session })
            .await
            .map_err(|e| UserError::from(e).into())
            .map(|resp| match resp {
                Ok(v) => HttpResponse::Ok().json(v),
                Err(e) => e.error_response(),
            })
    } else {
        Ok(HttpResponse::Ok().json(LoginState::NotLoggedIn))
    }
}

pub async fn totp(data: web::Json<TOTPValue>, identity: Identity) -> Result {
    let data = data.into_inner();
    let session = identity.id()?;
    let res = UserService::from_registry()
        .send(LoginTOTP {
            session,
            totp: data,
        })
        .await?;
    let v: LoginState = match res {
        Err(e) => return Err(APIError::from(e)),
        Ok(v) => v,
    };

    Ok(match &v {
        LoginState::LoggedIn(_) => HttpResponse::Accepted().json(v),
        LoginState::NotLoggedIn => HttpResponse::Forbidden().json(v),
        LoginState::RequiresTOTP => HttpResponse::Ok().json(v),
        LoginState::RequiresTOTPSetup(_) => HttpResponse::Ok().json(v),
    })
}

pub async fn login(
    request: HttpRequest,
    data: web::Json<Login>,
    identity: Option<Identity>,
) -> Result {
    let data = data.into_inner();
    if let Some(identity) = identity {
        let session = identity.id()?;
        let res = UserService::from_registry()
            .send(CheckSession {
                session: session.clone(),
            })
            .await?;
        match res {
            Err(e) => return Err(APIError::from(e)),
            Ok(v) => match v {
                LoginState::LoggedIn(_) => Ok(HttpResponse::BadRequest().json(v)),
                _ => login_core(session, data).await,
            },
        }
    } else {
        let identity = Identity::login(&request.extensions(), nanoid!(64)).map_err(|_| {
            UserError::InternalError("failed to perform identity::login".to_owned())
        })?;
        let session = identity.id()?;
        login_core(session, data).await
    }
}

pub async fn output(item: web::Path<ServiceRequest>, identity: Identity) -> Result {
    let service = item.into_inner().service;
    assert_perm!(identity, service, ServicePerm::OUTPUT);
    ServiceController::from_registry()
        .send(unchecked::GetOutput { id: service })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}

pub async fn services(identity: Identity) -> Result {
    let session = identity.id()?;
    ServiceController::from_registry()
        .send(GetSessionServices { session })
        .await
        .map_err(|e| UserError::from(e).into())
        .map(|response| match response {
            Ok(v) => HttpResponse::Ok().json(v),
            Err(e) => e.error_response(),
        })
}
