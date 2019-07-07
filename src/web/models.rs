use crate::crypto;
use crate::db::models as dbmodels;
pub use crate::db::models::{Session, SID, UID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ServiceRequest {
    pub service: SID,
}

#[derive(Debug, Deserialize)]
pub struct UserRequest {
    pub user: UID,
}

#[derive(Debug, Deserialize)]
pub struct PermRequest {
    pub service: SID,
    pub user: UID,
}

#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub name: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TOTP {
    pub secret: String,
    pub mode: dbmodels::TOTP_Mode,
    pub digits: u32,
}

impl From<dbmodels::TOTP> for TOTP {
    fn from(totp: dbmodels::TOTP) -> Self {
        Self {
            secret: crypto::totp_encode_secret(totp.secret.as_ref()),
            mode: totp.mode,
            digits: totp.digits,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserMin {
    pub name: String,
    pub id: UID,
    pub email: String,
}

impl From<dbmodels::FullUser> for UserMin {
    fn from(user: dbmodels::FullUser) -> Self {
        Self {
            name: user.name,
            id: user.id,
            email: user.email,
        }
    }
}

/// Login state sent via API
#[derive(Debug, Serialize)]
pub enum LoginState {
    /// Success
    LoggedIn(String),
    /// Invalid credentials
    NotLoggedIn,
    /// totp-login required
    RequiresTOTP,
    /// totp-setup required
    RequiresTOTPSetup(TOTP),
}

#[derive(Debug, Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}

pub type TOTPValue = u64;

#[derive(Debug, Serialize)]
pub struct CreateUserResp {
    pub uid: UID,
}
