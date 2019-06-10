use super::service::ControllerError;
use crate::handler::user;
use crate::settings::Service;
use crate::web::models::*;
use actix::prelude::*;
use serde::Serialize;

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
    pub session: String,
}

impl Message for LoginUser {
    type Result = Result<LoginState, user::Error>;
}

pub struct LogoutUser {
    pub session: String,
}

impl Message for LogoutUser {
    type Result = Result<(), user::Error>;
}

pub struct CreateUser {
    pub invoker: UID,
    pub user: NewUser,
}

impl Message for CreateUser {
    type Result = Result<CreateUserState, user::Error>;
}

pub struct EditUser {
    pub invoker: UID,
    pub user_uid: UID,
    pub data: EditUserData,
}

impl Message for EditUser {
    type Result = Result<bool,user::Error>;
}

#[derive(PartialEq)]
pub enum EditUserData {
    Name(String),
    Permission(Vec<String>),
    Password(String),
    TOTP(String),
}