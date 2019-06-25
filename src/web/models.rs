use crate::crypto;
use crate::db::models as dbmodels;
use serde::{Deserialize, Serialize};

pub type UID = i32;

#[derive(Debug, Deserialize)]
pub struct ServiceRequest {
    pub service: usize,
}

#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub name: String,
    pub password: String,
    pub email: String,
}

/// new type to make sure NewUser isn't passed with a raw password
#[derive(Debug)]
pub struct NewUserEncrypted {
    pub name: String,
    pub password_enc: String,
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
pub struct MinUser {
    pub name: String,
    pub id: UID,
    pub email: String,
}

/// Login state sent via API
#[derive(Debug, Serialize)]
pub enum LoginState {
    /// Success
    LoggedIn,
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
pub enum CreateUserState {
    Success(UID),
    EMailClaimed,
}
