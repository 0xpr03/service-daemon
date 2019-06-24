pub use crate::web::models::{MinUser, NewUser, UID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FullUser {
    pub name: String,
    pub id: UID,
    pub password: String,
    pub email: String,
    // to be used
    pub verified: bool,
    pub totp_secret: TOTP,
    /// TOTP setup complete
    pub totp_complete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TOTP {
    pub secret: Vec<u8>,
    pub mode: TOTP_Mode,
    pub digits: u32,
}

/// Wrapper for oath::HashType due to missing serde
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum TOTP_Mode {
    SHA1 = 0,
    SHA256 = 1,
    SHA512 = 2,
}

#[allow(non_snake_case)]
impl Into<oath::HashType> for TOTP_Mode {
    fn into(self) -> oath::HashType {
        match self {
            SHA1 => oath::HashType::SHA1,
            SHA256 => oath::HashType::SHA256,
            SHA512 => oath::HashType::SHA512,
        }
    }
}

impl From<oath::HashType> for TOTP_Mode {
    fn from(mode: oath::HashType) -> Self {
        match mode {
            oath::HashType::SHA1 => TOTP_Mode::SHA1,
            oath::HashType::SHA256 => TOTP_Mode::SHA256,
            oath::HashType::SHA512 => TOTP_Mode::SHA512,
        }
    }
}

pub type UserPermissions = Vec<String>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveLogin {
    pub id: UID,
    pub state: LoginState,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoginState {
    Missing_2FA,
    Complete,
    Requires_2FA_Setup,
}

pub type SessionPrivateKey = Vec<u8>;
