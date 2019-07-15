use super::error::*;
use crate::db::models::{ManagementPerm, ServicePerm};
use crate::handler::service::{LogType, State};
use crate::settings::Service;
use crate::web::models::*;
use actix::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct ServiceState {
    pub id: SID,
    pub name: String,
    pub state: State,
    pub uptime: u64,
}

/// Minimal service representation
#[derive(Serialize)]
pub struct ServiceMin {
    pub id: SID,
    pub name: String,
}

/// Check session for login state
#[derive(Message)]
#[rtype(result = "Result<LoginState, UserError>")]
pub struct CheckSession {
    pub session: Session,
}

/// Login user - password step
#[derive(Message)]
#[rtype(result = "Result<LoginState, UserError>")]
pub struct LoginUser {
    pub email: String,
    pub password: String,
    pub session: Session,
}

/// Login user - 2FA step
#[derive(Message)]
#[rtype(result = "Result<LoginState, UserError>")]
pub struct LoginTOTP {
    pub session: Session,
    pub totp: u64,
}

/// Logout user
#[derive(Message)]
#[rtype(result = "Result<(), UserError>")]
pub struct LogoutUser {
    pub session: Session,
}

/// Create a new user, checked
#[derive(Message)]
#[rtype(result = "Result<CreateUserResp, UserError>")]
pub struct CreateUser {
    pub invoker: Session,
    pub user: NewUser,
}

/// Delete a user, checked
#[derive(Message)]
#[rtype(result = "Result<(), UserError>")]
pub struct DeleteUser {
    pub invoker: Session,
    pub user: UID,
}

/// Get services of session, internal
#[derive(Message)]
#[rtype(result = "Result<Vec<SID>, UserError>")]
pub struct GetSessionServiceIDs {
    pub session: Session,
}

/// Get permissions of session for management
/// Returns error if no valid session is found
#[derive(Message)]
#[rtype(result = "Result<ManagementPerm, UserError>")]
pub struct GetManagementPerm {
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

/// Get all ServiceMin representations of services a use has access to
#[derive(Message)]
#[rtype(result = "Result<Vec<ServiceState>, ControllerError>")]
pub struct GetSessionServices {
    pub session: Session,
}

#[derive(Message)]
#[rtype(result = "Result<bool, UserError>")]
pub struct EditUser {
    pub invoker: UID,
    pub user_uid: UID,
    pub data: EditUserData,
}

#[derive(PartialEq)]
#[allow(unused)]
pub enum EditUserData {
    Name(String),
    Mail(String),
    ServicePermission((SID, ServicePerm)),
    Password(String),
    // TOTP(String),
}

/// Unchecked commands, part of the internal API and should not be callable without authentification checks.
pub mod unchecked {
    use super::*;
    use std::collections::HashMap;

    /// **Unchecked!** Set permissions of user for service  
    /// For administration
    #[derive(Message)]
    #[rtype(result = "Result<(), UserError>")]
    pub struct SetServicePermUser {
        pub user: UID,
        pub service: SID,
        pub perm: ServicePerm,
    }

    /// **Unchecked!** Get permissions of user for service  
    /// For administration
    #[derive(Message)]
    #[rtype(result = "Result<ServicePerm, UserError>")]
    pub struct GetServicePermUser {
        pub user: UID,
        pub service: SID,
    }

    /// **Unchecked!** send stdin to service
    #[derive(Message)]
    #[rtype(result = "Result<(), ControllerError>")]
    pub struct SendStdin {
        pub id: SID,
        pub input: String,
    }

    /// **Unchecked!** internal, set password cost for future passwords  
    /// For startup
    #[derive(Message)]
    pub struct SetPasswordCost {
        pub cost: u32,
    }

    /// **Unchecked!** start service
    #[derive(Message)]
    #[rtype(result = "Result<(), ControllerError>")]
    pub struct StartService {
        pub id: SID,
    }

    /// **Unchecked!** stop service
    #[derive(Message)]
    #[rtype(result = "Result<(), ControllerError>")]
    pub struct StopService {
        pub id: SID,
    }

    /// **Unchecked!** kill service
    #[derive(Message)]
    #[rtype(result = "Result<(), ControllerError>")]
    pub struct KillService {
        pub id: SID,
    }

    /// **Unchecked!** get service status
    #[derive(Message)]
    #[rtype(result = "Result<ServiceState, ControllerError>")]
    pub struct GetServiceState {
        pub id: SID,
    }

    /// **Unchecked!** get service output
    #[derive(Message)]
    #[rtype(result = "Result<Vec<LogType<String>>, ControllerError>")]
    pub struct GetOutput {
        pub id: SID,
    }

    /// **Unchecked!** internal, signal service state change  
    /// For service internal use.
    #[derive(Message)]
    pub struct ServiceStateChanged {
        pub id: SID,
        pub running: bool,
    }

    /// **Unchecked!** internal, startup check  
    /// For startup
    #[derive(Message)]
    #[rtype(result = "Result<(), UserError>")]
    pub struct StartupCheck {}

    /// **Unchecked!** internal, load services  
    /// For startup
    #[derive(Message)]
    pub struct LoadServices {
        pub data: Vec<Service>,
    }

    /// **Unchecked!** get all service SIDs  
    /// For administration
    #[derive(Message)]
    #[rtype(result = "Result<Vec<SID>, ControllerError>")]
    pub struct GetServiceIDs {}

    /// **Unchecked!** get all services as ServiceMin representation
    #[derive(Message)]
    #[rtype(result = "Result<Vec<ServiceMin>, ControllerError>")]
    pub struct GetAllServicesMin {}

    /// **Unchecked!** get all users as UserMin representation
    #[derive(Message)]
    #[rtype(result = "Result<Vec<UserMin>, UserError>")]
    pub struct GetAllUsers {}

    /// **Unchecked!** get all SIDs with the permissions of the specified user
    #[derive(Message)]
    #[rtype(result = "Result<HashMap<SID,SPMin>, ControllerError>")]
    pub struct GetUserServicePermsAll {
        pub user: UID,
    }

    /// Minimal service permission representation
    #[derive(Serialize)]
    pub struct SPMin {
        pub id: SID,
        pub name: String,
        pub has_perm: bool,
    }

    /// **Unchecked!** get user info  
    /// For administration
    #[derive(Message)]
    #[rtype(result = "Result<UserMin, UserError>")]
    pub struct GetUserInfo {
        pub user: UID,
    }

    /// **Unchecked!** set user info  
    /// For administration
    #[derive(Message)]
    #[rtype(result = "Result<(), UserError>")]
    pub struct SetUserInfo {
        pub user: UserMin,
    }
}
