pub use crate::web::models::UserMin;
use bitflags::bitflags;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

/// User ID
pub type UID = i32;
/// Permission ID
pub type SID = u32;
/// User login session
pub type Session = String;

/// new type to make sure NewUser isn't passed with a raw password that easy
#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct NewUserEnc {
    pub name: String,
    pub password_enc: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct FullUser {
    pub name: String,
    pub id: UID,
    pub password: String,
    pub email: String,
    // to be used
    pub verified: bool,
    pub totp: TOTP,
    /// TOTP setup complete
    pub totp_setup_complete: bool,
    pub admin: bool,
}

pub type ConsoleOutput = Vec<ConsoleType<String>>;

#[derive(Serialize, Deserialize)]
pub enum ConsoleType<T> {
    Stdin(T),
    Stdout(T),
    Stderr(T),
    State(T),
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, PartialEq))]
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

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct ServicePerm: u32 {
        /// Start service
        const START  = 0b0000_0001;
        /// Stop service
        const STOP   = 0b0000_0010;
        /// Stdin write all
        const STDIN_ALL = 0b0000_0100;
        /// Output inspect
        const OUTPUT = 0b0000_1000;
        /// Kill service
        const KILL   = 0b0001_0000;
        /// Log inspection
        const LOG    = 0b0010_0000;
    }
}

impl Default for ServicePerm {
    fn default() -> Self {
        Self::empty()
    }
}

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

/// LogEntry with Invoker entry instead of ID
#[derive(Debug, Serialize)]
pub struct LogEntryResolved {
    pub time: Date,
    pub action: LogAction,
    pub invoker: Option<Invoker>,
    pub id: LogID,
    pub console_log: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct Invoker {
    pub id: UID,
    pub name: String,
}

impl From<FullUser> for Invoker {
    fn from(user: FullUser) -> Self {
        Self {
            id: user.id,
            name: user.name,
        }
    }
}

/// Full log entry with unique key
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: Date,
    pub action: LogAction,
    pub invoker: Option<UID>,
    pub log_id: LogID,
    /// true if console log exists
    pub console_log: bool,
}

impl LogEntry {
    pub fn new(log_id: LogID, entry: NewLogEntry, console_log: bool) -> Self {
        Self {
            time: entry.time,
            action: entry.action,
            invoker: entry.invoker,
            log_id,
            console_log: console_log,
        }
    }
}

/// Log entry without unique key, which is created by DB
#[derive(Debug)]
pub struct NewLogEntry {
    pub time: Date,
    pub action: LogAction,
    pub invoker: Option<UID>,
}

impl NewLogEntry {
    pub fn new(action: LogAction, invoker: Option<UID>) -> Self {
        Self {
            time: Utc::now().timestamp_millis(), // TODO
            action,
            invoker,
        }
    }
}

/// Logged action
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum LogAction {
    SystemStartup,
    ServiceMaxRetries(usize),
    ServiceCmdKilled,
    ServiceKilled,
    ServiceCmdStop,
    ServiceEnded,
    ServiceStopped,
    ServiceStartFailed(String),
    ServiceStarted,
    ServiceCmdStart,
    ServiceCrashed(i32),
    Stdin(String),
}

pub type Date = i64;
pub type LogID = u64;
