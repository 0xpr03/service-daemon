use crate::db;
use crate::web::models::SID;
use actix_web::{error::ResponseError, HttpResponse};
use bcrypt::BcryptError;

#[derive(Fail, Debug)]
pub enum UserError {
    #[fail(display = "Internal DB error {}", _0)]
    DBError(db::Error),
    #[fail(display = "Error with password hashing! {}", _0)]
    HashError(#[cause] BcryptError),
    #[fail(display = "Lacking permissions!")]
    InvalidPermissions,
    #[fail(display = "Special internal error: {}", _0)]
    InternalError(String),
    #[fail(display = "Invalid session for operation")]
    InvalidSession,
}

impl ResponseError for UserError {
    fn error_response(&self) -> HttpResponse {
        match self {
            UserError::InvalidPermissions => HttpResponse::Unauthorized().json("unauthorized"),
            UserError::InvalidSession => HttpResponse::Unauthorized().json("invalid_session"),
            _ => {
                HttpResponse::InternalServerError().json("Internal Server Error, Please try later")
            }
        }
    }
}

impl From<bcrypt::BcryptError> for UserError {
    fn from(error: bcrypt::BcryptError) -> Self {
        UserError::HashError(error)
    }
}
impl From<db::Error> for UserError {
    fn from(error: db::Error) -> Self {
        UserError::DBError(error)
    }
}

#[derive(Fail, Debug)]
pub enum ControllerError {
    #[fail(display = "Failed to load services from data, services already loaded!")]
    ServicesNotEmpty,
    #[fail(display = "Invalid instance ID: {}", _0)]
    InvalidInstance(SID),
    #[fail(display = "Unable to start, IO error: {}", _0)]
    StartupIOError(::std::io::Error),
    #[fail(display = "Service is stopped!")]
    ServiceStopped,
    #[fail(display = "Unable to execute, missing service handles! This is a bug!")]
    NoServiceHandle,
    #[fail(display = "Service already running!")]
    ServiceRunning,
    #[fail(display = "Pipe to process is broken! This is an bug!")]
    BrokenPipe,
}

impl ResponseError for ControllerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ControllerError::InvalidInstance(_) => {
                HttpResponse::BadRequest().body("invalid instance")
            }
            ControllerError::ServiceRunning => {
                HttpResponse::Conflict().body("Instance not running!")
            }
            ControllerError::ServiceStopped => {
                HttpResponse::Conflict().body("Instance already running!")
            }
            ControllerError::BrokenPipe => HttpResponse::InternalServerError().body("Broken pipe!"),
            v => {
                error!("Controller error {}!", v);
                HttpResponse::InternalServerError().body("Internal Server Error, Please try later")
            }
        }
    }
}
