
pub use crate::web::models::{MinUser,NewUser,UID};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct FullUser {
    pub name: String,
    pub id: UID,
    pub password: String,
    pub totp_secret: Option<String>,
}

pub type UserPermissions = Vec<String>;

#[derive(Serialize, Deserialize)]
pub struct ActiveLogin {
    pub id: UID,
    pub incomplete: bool,
}