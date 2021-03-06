use super::error::*;
use crate::db::models::{ConsoleOutput, LogEntryResolved, LogID, ServicePerm};
use crate::handler::service::State;
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

/// Set a new password, requires admin or the old password
#[derive(Message)]
#[rtype(result = "Result<(), UserError>")]
pub struct SetUserPassword {
    pub data: SetPassword,
    pub invoker: Session,
    pub id: UID,
}

/// Reset user TOTP, requires admin or the current password
#[derive(Message)]
#[rtype(result = "Result<(), UserError>")]
pub struct ResetUserTOTP {
    pub data: ResetTOTP,
    pub invoker: Session,
    pub id: UID,
}

/// Set user info, requires user to be admin when requesting for foreign UID
#[derive(Message)]
#[rtype(result = "Result<(), UserError>")]
pub struct SetUserInfo {
    pub data: UserMinData,
    pub user: UID,
    pub invoker: Session,
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

/// Get permissions of session for administration
/// Returns error if no valid session is found
#[derive(Message)]
#[rtype(result = "Result<bool, UserError>")]
pub struct GetAdminPerm {
    pub session: Session,
}

/// Get permissions & UID of session for service
/// Returns error if no valid session is found
#[derive(Message)]
#[rtype(result = "Result<(UID,ServicePerm), UserError>")]
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
        /// Invoker to use for logging
        pub user: Option<UID>,
    }

    /// **Unchecked!** internal, set user controller settings  
    /// For startup
    #[derive(Message)]
    #[rtype(result = "()")]
    pub struct SetConfig {
        pub cost: u32,
        pub max_session_age_secs: u32,
        pub disable_totp: bool,
    }

    /// **Unchecked!** start service
    #[derive(Message)]
    #[rtype(result = "Result<(), ControllerError>")]
    pub struct StartService {
        pub id: SID,
        /// Invoker to use for logging
        pub user: Option<UID>,
    }

    /// **Unchecked!** stop service
    #[derive(Message)]
    #[rtype(result = "Result<(), ControllerError>")]
    pub struct StopService {
        pub id: SID,
        /// Invoker to use for logging
        pub user: Option<UID>,
    }

    /// **Unchecked!** kill service
    #[derive(Message)]
    #[rtype(result = "Result<(), ControllerError>")]
    pub struct KillService {
        pub id: SID,
        /// Invoker to use for logging
        pub user: Option<UID>,
    }

    /// **Unchecked!** get service status
    #[derive(Message)]
    #[rtype(result = "Result<ServiceState, ControllerError>")]
    pub struct GetServiceState {
        pub id: SID,
    }

    /// **Unchecked!** get service latest log
    #[derive(Message)]
    #[rtype(result = "Result<Vec<LogEntryResolved>, ControllerError>")]
    pub struct GetLogLatest {
        pub id: SID,
        pub amount: usize,
    }

    /// **Unchecked!** get service log console data
    #[derive(Message)]
    #[rtype(result = "Result<ConsoleOutput, ControllerError>")]
    pub struct GetLogConsole {
        pub id: SID,
        pub log_id: LogID,
    }

    /// **Unchecked!** get service log details
    #[derive(Message)]
    #[rtype(result = "Result<LogEntryResolved, ControllerError>")]
    pub struct GetLogDetails {
        pub id: SID,
        pub log_id: LogID,
    }

    /// **Unchecked!** get service output
    #[derive(Message)]
    #[rtype(result = "Result<ConsoleOutput, ControllerError>")]
    pub struct GetOutput {
        pub id: SID,
    }

    /// **Unchecked!** internal, signal service state change  
    /// For service internal use.
    #[derive(Message)]
    #[rtype(result = "()")]
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
    #[rtype(result = "()")]
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
}
