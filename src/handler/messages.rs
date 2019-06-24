use super::error::*;
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
#[rtype(result = "Result<(), UserError>")]
pub struct StartupCheck {}

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

pub struct StopService {
    pub id: usize,
}

impl Message for StopService {
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

pub struct SendStdin {
    pub id: usize,
    pub input: String,
}

impl Message for SendStdin {
    type Result = Result<(), ControllerError>;
}

#[derive(Serialize)]
pub struct ServiceMin {
    pub id: usize,
    pub name: String,
    pub running: bool,
}

pub struct CheckSession {
    pub session: String,
}

impl Message for CheckSession {
    type Result = Result<LoginState, UserError>;
}

pub struct LoginUser {
    pub email: String,
    pub password: String,
    pub session: String,
}

impl Message for LoginUser {
    type Result = Result<LoginState, UserError>;
}

pub struct LogoutUser {
    pub session: String,
}

impl Message for LogoutUser {
    type Result = Result<(), UserError>;
}

pub struct CreateUser {
    pub invoker: UID,
    pub user: NewUser,
}

impl Message for CreateUser {
    type Result = Result<CreateUserState, UserError>;
}

pub struct EditUser {
    pub invoker: UID,
    pub user_uid: UID,
    pub data: EditUserData,
}

impl Message for EditUser {
    type Result = Result<bool, UserError>;
}

#[derive(PartialEq)]
pub enum EditUserData {
    Name(String),
    Mail(String),
    Permission(Vec<String>),
    Password(String),
    // TOTP(String),
}
