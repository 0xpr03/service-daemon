use crate::db;
use crate::web::models::SID;
use actix::MailboxError;
use actix_web::{error::ResponseError, HttpResponse};
use bcrypt::BcryptError;
use actix_threadpool::BlockingError;

#[derive(Fail, Debug)]
pub enum StartupError {
    #[fail(display = "Error when accessing controller: {}", _0)]
    ControllerError(#[cause] ControllerError),
    #[fail(display = "Error when accessing UserController {}", _0)]
    UserError(#[cause] UserError),
    #[fail(display = "Error when accessing resource: {}", _0)]
    SendError(#[cause] MailboxError),
}

impl From<ControllerError> for StartupError {
    fn from(error: ControllerError) -> Self {
        StartupError::ControllerError(error)
    }
}

impl From<UserError> for StartupError {
    fn from(error: UserError) -> Self {
        StartupError::UserError(error)
    }
}

impl From<MailboxError> for StartupError {
    fn from(error: MailboxError) -> Self {
        StartupError::SendError(error)
    }
}

#[derive(Fail, Debug)]
pub enum UserError {
    #[fail(display = "Internal DB error: {}", _0)]
    DBError(db::Error),
    #[fail(display = "Error with password hashing! {}", _0)]
    HashError(#[cause] BcryptError),
    #[fail(display = "Lacking permissions!")]
    InvalidPermissions,
    #[fail(display = "Special internal error: {}", _0)]
    InternalError(String),
    #[fail(display = "Invalid session for operation")]
    InvalidSession,
    #[fail(display = "Error when accessing resource: {}", _0)]
    SendError(#[cause] MailboxError),
    #[fail(display = "Email already in use")]
    EmailInUse,
    #[fail(display = "Invalid password for action!")]
    InvalidPassword,
    #[fail(display = "Invalid data: {}", _0)]
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
            BlockingError::Canceled => UserError::InternalError(String::from("BCrypt blocking process canceled!")),
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
        match error {
            db::Error::EMailExists => UserError::EmailInUse,
            v => UserError::DBError(v),
        }
    }
}

impl From<MailboxError> for UserError {
    fn from(error: MailboxError) -> Self {
        UserError::SendError(error)
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
    #[fail(display = "Stdin pipe to process is broken! This is an bug!")]
    BrokenPipe,
    #[fail(display = "Error when accessing UserController: {}", _0)]
    UserError(#[cause] UserError),
    #[fail(display = "Error when accessing resource: {}", _0)]
    SendError(#[cause] MailboxError),
    #[fail(display = "Internal DB error: {}", _0)]
    DBError(db::Error),
    #[fail(display = "Service has no soft-stop parameter")]
    NoSoftStop,
}

impl From<db::Error> for ControllerError {
    fn from(error: db::Error) -> Self {
        ControllerError::DBError(error)
    }
}

impl From<UserError> for ControllerError {
    fn from(error: UserError) -> Self {
        ControllerError::UserError(error)
    }
}

impl From<MailboxError> for ControllerError {
    fn from(error: MailboxError) -> Self {
        ControllerError::SendError(error)
    }
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
                error!("{}", v);
                HttpResponse::InternalServerError().body("Internal Server Error, Please try later")
            }
        }
    }
}
