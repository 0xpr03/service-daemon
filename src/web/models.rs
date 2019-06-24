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

#[derive(Debug, Serialize)]
pub struct MinUser {
    pub name: String,
    pub id: UID,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub enum LoginState {
    /// Success
    LoggedIn,
    /// Invalid credentials
    NotLoggedIn,
    /// totp-login required
    Requires_TOTP,
    /// totp-setup required
    Requires_TOTP_Setup,
}

#[derive(Debug, Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub enum CreateUserState {
    Success(UID),
    EMailClaimed,
}
