use serde::Deserialize;

pub type UID = i32;

#[derive(Deserialize)]
pub struct ServiceRequest {
    pub service: usize
}

#[derive(Deserialize)]
pub struct NewUser {
    pub name: String,
}