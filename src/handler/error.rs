use crate::db;
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
}

impl ResponseError for UserError {
    fn error_response(&self) -> HttpResponse {
        match self {
            UserError::InvalidPermissions => HttpResponse::Unauthorized().json("Unauthorized"),
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
