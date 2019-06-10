use super::service::ControllerError;
use crate::handler::user;
use crate::settings::Service;
use crate::web::models::*;
use actix::prelude::*;
use serde::Serialize;

// #[derive(Message)]
// pub struct Stdout {
//     pub id: usize,
//     pub data: String,
// }

#[derive(Message)]
pub struct ServiceStateChanged {
    pub id: usize,
    pub running: bool,
}

#[derive(Message)]
pub struct LoadServices {
    pub data: Vec<Service>,
}

pub struct StartService {
    pub id: usize,
}

impl Message for StartService {
    type Result = Result<(), ControllerError>;
}

pub struct Stop {
    pub id: usize,
}

impl Message for Stop {
    type Result = Result<(), ControllerError>;
}

pub struct GetOutput {
    pub id: usize,
}

impl Message for GetOutput {
    type Result = Result<String, ControllerError>;
}

pub struct GetServices {}

impl Message for GetServices {
    type Result = Result<Vec<ServiceMin>, ControllerError>;
}

#[derive(Serialize)]
pub struct ServiceMin {
    pub id: usize,
    pub name: String,
    pub running: bool,
}

pub struct LoginUser {
    pub name: String,
    pub password: String,
    pub identity: String,
}

impl Message for LoginUser {
    type Result = Result<LoginState, user::Error>;
}

pub struct LoginUserResponse {
    pub code: i32,
    pub msg: String,
    pub success: bool,
    pub id: UID,
}

#[derive(Message)]
pub struct LogoutUser {
    pub id: i32,
}

pub struct CreateUser {
    pub id: UID,
    pub user: NewUser,
}

impl Message for CreateUser {
    type Result = Result<CreateUserResponse, ControllerError>;
}

pub struct CreateUserResponse {
    pub code: i32,
    pub msg: String,
    pub success: bool,
    pub id: Option<UID>,
    pub password: Option<String>,
}
