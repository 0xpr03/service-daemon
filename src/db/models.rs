pub use crate::web::models::{MinUser};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// User ID
pub type UID = i32;
/// Permission ID
pub type SID = u32;

/// new type to make sure NewUser isn't passed with a raw password that easy
#[derive(Debug)]
#[cfg_attr(test, derive(Clone,PartialEq))]
pub struct NewUserEnc {
    pub name: String,
    pub password_enc: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone,PartialEq))]
pub struct FullUser {
    pub name: String,
    pub id: UID,
    pub password: String,
    pub email: String,
    // to be used
    pub verified: bool,
    pub totp: TOTP,
    /// TOTP setup complete
    pub totp_complete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone,PartialEq))]
pub struct TOTP {
    pub secret: Vec<u8>,
    pub mode: TOTP_Mode,
    pub digits: u32,
}

/// Wrapper for oath::HashType due to missing serde
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, PartialEq))]
#[allow(non_camel_case_types)]
pub enum TOTP_Mode {
    SHA1 = 0,
    SHA256 = 1,
    SHA512 = 2,
}

#[allow(non_snake_case)]
impl Into<oath::HashType> for TOTP_Mode {
    fn into(self) -> oath::HashType {
        self.as_HashType()
    }
}

#[allow(non_snake_case)]
impl TOTP_Mode {
    pub fn as_HashType(&self) -> oath::HashType {
        match self {
            TOTP_Mode::SHA1 => oath::HashType::SHA1,
            TOTP_Mode::SHA256 => oath::HashType::SHA256,
            TOTP_Mode::SHA512 => oath::HashType::SHA512,
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

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct ManagementPerm {
    pub admin: bool,
}

impl Default for ManagementPerm {
    fn default() -> Self {
        Self { admin: false }
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct ServicePerm: u32 {
        /// Start service
        const START = 0b00000001;
        /// Stop service
        const STOP  = 0b00000010;
        /// Stdin write all
        const STDIN_ALL = 0b00000100;
        /// Stdin write specific commands
        const STDIN = 0b00001000;
        /// Stdout inspect
        const STDOUT = 0b00010000;
        /// Stderr inspect
        const STDERR = 0b00100000;
    }
}

impl Default for ServicePerm {
    fn default() -> Self {
        Self::empty()
    }
}

/// Specific commands a user can use
pub type StdinCommands = Vec<String>;

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct ActiveLogin {
    pub id: UID,
    pub state: LoginState,
}

/// Login state stored internally, doesn't have "not logged in"
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub enum LoginState {
    Missing2Fa,
    Complete,
    Requires2FaSetup,
}
