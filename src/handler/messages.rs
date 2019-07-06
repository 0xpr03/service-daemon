use super::error::*;
use crate::db::models::ServicePerm;
use crate::handler::service::{LogType,State};
use crate::settings::Service;
use crate::web::models::*;
use actix::prelude::*;
use serde::Serialize;

#[derive(Message)]
pub struct ServiceStateChanged {
    pub id: SID,
    pub running: bool,
}

#[derive(Message)]
#[rtype(result = "Result<(), UserError>")]
pub struct StartupCheck {}

#[derive(Message)]
pub struct LoadServices {
    pub data: Vec<Service>,
}

#[derive(Message)]
pub struct SetPasswordCost {
    pub cost: u32,
}

pub struct StartService {
    pub id: SID,
}

impl Message for StartService {
    type Result = Result<(), ControllerError>;
}

pub struct StopService {
    pub id: SID,
}

impl Message for StopService {
    type Result = Result<(), ControllerError>;
}

#[derive(Message)]
#[rtype(result = "Result<ServiceState, ControllerError>")]
pub struct GetServiceState {
    pub id: SID,
}

#[derive(Serialize)]
pub struct ServiceState {
    pub name: String,
    pub state: State,
    pub uptime: u64,
}

#[derive(Message)]
#[rtype(result = "Result<Vec<LogType<String>>, ControllerError>")]
pub struct GetOutput {
    pub id: SID,
}

impl Message for GetServiceIDs {
    type Result = Result<Vec<SID>, ControllerError>;
}

pub struct GetServiceIDs {}

pub struct SendStdin {
    pub id: SID,
    pub input: String,
}

impl Message for SendStdin {
    type Result = Result<(), ControllerError>;
}

#[derive(Serialize)]
pub struct ServiceMin {
    pub id: SID,
    pub name: String,
    pub running: bool,
}

pub struct CheckSession {
    pub session: Session,
}

impl Message for CheckSession {
    type Result = Result<LoginState, UserError>;
}

pub struct LoginUser {
    pub email: String,
    pub password: String,
    pub session: Session,
}

impl Message for LoginUser {
    type Result = Result<LoginState, UserError>;
}

pub struct LoginTOTP {
    pub session: Session,
    pub totp: u64,
}

impl Message for LoginTOTP {
    type Result = Result<LoginState, UserError>;
}

pub struct LogoutUser {
    pub session: Session,
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

#[derive(Message)]
#[rtype(result = "Result<Vec<SID>, UserError>")]
pub struct GetUserServiceIDs {
    pub session: Session,
}

/// Get permissions of session for service
/// Returns error if no valid session is found
#[derive(Message)]
#[rtype(result = "Result<ServicePerm, UserError>")]
pub struct GetServicePerm {
    pub session: Session,
    pub service: SID,
}

#[derive(Message)]
#[rtype(result = "Result<Vec<ServiceMin>, ControllerError>")]
pub struct GetUserServices {
    pub session: Session,
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
    ServicePermission((SID, ServicePerm)),
    Password(String),
    // TOTP(String),
}
