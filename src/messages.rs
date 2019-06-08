use crate::service::ControllerError;
use crate::settings::Service;
use actix::prelude::*;

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
    type Result = Result<String, ControllerError>;
}
