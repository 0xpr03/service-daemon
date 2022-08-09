use crate::db;
use crate::web::models::SID;
use actix::MailboxError;
use actix_threadpool::BlockingError;
use actix_web::{error::ResponseError, HttpResponse};
use bcrypt::BcryptError;
use log::error;

#[derive(thiserror::Error, Debug)]
pub enum StartupError {
    #[error("Error when accessing controller: {0}")]
    ControllerError(#[from] ControllerError),
    #[error("Error when accessing UserController {0}")]
    UserError(#[from] UserError),
    #[error("Error when accessing resource: {0}")]
    SendError(#[from] MailboxError),
}

#[derive(thiserror::Error, Debug)]
pub enum UserError {
    #[error("Internal DB error: {0}")]
    DBError(db::Error),
    #[error("Error with password hashing! {0}")]
    HashError(#[from] BcryptError),
    #[error("Lacking permissions!")]
    InvalidPermissions,
    #[error("Special internal error: {0}")]
    InternalError(String),
    #[error("Invalid session for operation")]
    InvalidSession,
    #[error("Error when accessing resource: {0}")]
    SendError(#[from] MailboxError),
    #[error("Email already in use")]
    EmailInUse,
    #[error("Invalid password for action!")]
    InvalidPassword,
    #[error("Invalid data: {0}")]
    BadRequest(&'static str),
}

impl ResponseError for UserError {
    fn error_response(&self) -> HttpResponse {
        match self {
            UserError::EmailInUse => HttpResponse::Conflict().json("email_claimed"),
            UserError::InvalidPermissions => HttpResponse::Unauthorized().json("unauthorized"),
            UserError::InvalidSession => HttpResponse::Unauthorized().json("invalid_session"),
            UserError::InvalidPassword => HttpResponse::Unauthorized().json("invalid_password"),
            UserError::BadRequest(msg) => HttpResponse::BadRequest().json(msg),
            v => {
                error!("{}", v);
                HttpResponse::InternalServerError().json("Internal Server Error, Please try later")
            }
        }
    }
}

impl From<BlockingError<bcrypt::BcryptError>> for UserError {
    fn from(error: BlockingError<bcrypt::BcryptError>) -> Self {
        match error {
            BlockingError::Error(e) => UserError::HashError(e),
            BlockingError::Canceled => {
                UserError::InternalError(String::from("BCrypt blocking process canceled!"))
            }
        }
    }
}

impl From<db::Error> for UserError {
    fn from(error: db::Error) -> Self {
        match error {
            db::Error::EMailExists => UserError::EmailInUse,
            v => UserError::DBError(v),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ControllerError {
    #[error("Invalid log ID: {0}")]
    InvalidLog(crate::db::models::LogID),
    #[error("Invalid instance ID: {0}")]
    InvalidInstance(SID),
    #[error("Unable to start, IO error: {0}")]
    StartupIOError(::std::io::Error),
    #[error("Service is stopped!")]
    ServiceStopped,
    #[error("Unable to execute, missing service handles! This is a bug!")]
    NoServiceHandle,
    #[error("Service already running!")]
    ServiceRunning,
    #[error("Stdin pipe to process is broken! This is an bug!")]
    BrokenPipe,
    #[error("Error when accessing UserController: {0}")]
    UserError(#[from] UserError),
    #[error("Error when accessing resource: {0}")]
    SendError(#[from] MailboxError),
    #[error("Internal DB error: {0}")]
    DBError(#[from] db::Error),
    #[error("Service has no soft-stop parameter")]
    NoSoftStop,
    #[error("Service has no backoff handle!")]
    NoBackoffHandle,
}

impl ResponseError for ControllerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ControllerError::InvalidInstance(_) => {
                HttpResponse::BadRequest().body("invalid instance")
            }
            ControllerError::InvalidLog(_) => HttpResponse::BadRequest().body("invalid log"),
            ControllerError::ServiceRunning => {
                HttpResponse::MethodNotAllowed().body("Instance already running!")
            }
            ControllerError::ServiceStopped => {
                HttpResponse::MethodNotAllowed().body("Instance not running!")
            }
            ControllerError::NoSoftStop => HttpResponse::MethodNotAllowed().body("no soft stop available for this service"),
            ControllerError::UserError(u) => u.error_response(),
            ControllerError::BrokenPipe => HttpResponse::InternalServerError().body("Broken pipe!"),
            v => {
                error!("{}", v);
                HttpResponse::InternalServerError().body("Internal Server Error, Please try later")
            }
        }
    }
}
