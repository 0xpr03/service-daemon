use serde::{Deserialize, Serialize};

pub type UID = i32;

#[derive(Deserialize)]
pub struct ServiceRequest {
    pub service: usize,
}

#[derive(Deserialize)]
pub struct NewUser {
    pub name: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct MinUser {
    pub name: String,
    pub id: UID,
}

#[derive(Serialize)]
pub enum LoginState {
    LoggedIn,
    Failed,
    TOTP,
}
