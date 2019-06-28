use super::messages::*;

use super::error::UserError;
use crate::crypto::*;
use crate::db;
use crate::db::{
    models::*,
    DBInterface, DB,
};
use crate::web::models::{CreateUserState, LoginState, NewUser, UID};
use actix::prelude::*;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::iter;

const ROOT_NAME: &'static str = "Root";
const ROOT_EMAIL: &'static str = "root@localhost";
const ROOT_PASSWORD_LENGTH: usize = 20;
const CLEANUP_INTERVAL: u64 = 60 * 20; // seconds

/// Service for handling user related things
pub struct UserService {
    brcypt_cost: u32,
    login_max_age: u32,
}

type Result<T> = ::std::result::Result<T, UserError>;

impl UserService {
    fn cleanup_sessions(&mut self, _context: &mut Context<Self>) {
        match DB.delete_old_logins(self.login_max_age) {
            Ok(v) => debug!("Removed {} outdated logins.", v),
            Err(e) => warn!("Unable to remove old logins: {}", e),
        }
    }
    fn is_admin(&self, user: UID) -> Result<bool> {
        Ok(DB.get_perm_man(user)?.admin)
    }
    fn get_root_user(&self) -> Result<Option<FullUser>> {
        match DB.get_user(DB.get_root_id()) {
            Ok(user) => Ok(Some(user)),
            Err(db::Error::InvalidUser(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    fn create_user_unchecked(&self, user: NewUser) -> Result<CreateUserState> {
        let user_enc = NewUserEnc {
            email: user.email,
            name: user.name,
            password_enc: bcrypt_password(&user.password, self.brcypt_cost)?,
        };
        let v = DB.create_user(user_enc);
        match v {
            Err(e) => {
                if let db::Error::EMailExists(_) = e {
                    return Ok(CreateUserState::EMailClaimed);
                }
                Err(e.into())
            }
            Ok(user) => Ok(CreateUserState::Success(user.id)),
        }
    }
}

impl Default for UserService {
    fn default() -> Self {
        Self {
            brcypt_cost: 12,
            login_max_age: 3600,
        }
    }
}

impl Handler<StartupCheck> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, _msg: StartupCheck, _ctx: &mut Context<Self>) -> Result<()> {
        let user = self.get_root_user()?;
        let create_root = user.is_none();
        if let Some(user) = user {
            if !user.totp_complete {
                warn!("2FA setup incomplete for root!");
            }
        }

        if create_root {
            // TODO: use chacha20 once rand is compatible again with rand_chacha
            let mut rng = thread_rng();
            let password: String = iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(ROOT_PASSWORD_LENGTH)
                .collect();
            let start = std::time::Instant::now();
            match self.create_user_unchecked(NewUser {
                name: ROOT_NAME.to_string(),
                password: password.clone(),
                email: ROOT_EMAIL.to_string(),
            })? {
                CreateUserState::Success(uid) => {
                    assert_eq!(uid, DB.get_root_id());
                    let end = start.elapsed().as_millis();

                    DB.set_perm_man(uid, &ManagementPerm { admin: true })?;

                    if end > 2_000 {
                        warn!(
                            "Took {} ms to encrypt password using current configuration!",
                            end
                        );
                    } else {
                        info!("Took {} ms to encrypt password.", end)
                    }
                    info!("Created root user:");
                    info!("Email: {} Passwort: {}", ROOT_EMAIL, password);
                    info!("Please login to setup 2FA!");
                }
                v => {
                    return Err(UserError::InternalError(format!(
                        "Couldn't create root user: {:?}",
                        v
                    )))
                }
            }
        }
        Ok(())
    }
}

impl Handler<LoginTOTP> for UserService {
    type Result = Result<LoginState>;

    fn handle(&mut self, msg: LoginTOTP, _ctx: &mut Context<Self>) -> Self::Result {
        let mut login = match DB.get_login(&msg.session, self.login_max_age)? {
            Some(v) => v,
            None => return Ok(LoginState::NotLoggedIn),
        };
        let mut user = DB.get_user(login.id)?;
        let expected_totp = totp_calculate(&user.totp);
        if expected_totp == msg.totp {
            login.state = db::models::LoginState::Complete;
            DB.set_login(&msg.session, Some(login))?;
            if !user.totp_complete {
                user.totp_complete = true;
                DB.update_user(user)?;
            }
            Ok(LoginState::LoggedIn)
        } else {
            Ok(match user.totp_complete {
                true => LoginState::RequiresTOTP,
                false => LoginState::RequiresTOTPSetup(user.totp.into()),
            })
        }
    }
}

impl Handler<LoginUser> for UserService {
    type Result = Result<LoginState>;

    fn handle(&mut self, msg: LoginUser, _ctx: &mut Context<Self>) -> Self::Result {
        DB.set_login(&msg.session, None)?;
        let uid = match DB.get_id_by_email(&msg.email)? {
            Some(v) => v,
            None => return Ok(LoginState::NotLoggedIn),
        };
        let user = DB.get_user(uid)?;
        if bcrypt_verify(&msg.password, &user.password)? {
            let state = match user.totp_complete {
                true => db::models::LoginState::Missing2Fa,
                false => db::models::LoginState::Requires2FaSetup,
            };
            DB.set_login(&msg.session, Some(ActiveLogin { state, id: uid }))?;
            if user.totp_complete {
                Ok(LoginState::RequiresTOTP)
            } else {
                Ok(LoginState::RequiresTOTPSetup(user.totp.into()))
            }
        } else {
            DB.set_login(&msg.session, None)?;
            Ok(LoginState::NotLoggedIn)
        }
    }
}

impl Handler<CheckSession> for UserService {
    type Result = Result<LoginState>;

    fn handle(&mut self, msg: CheckSession, _ctx: &mut Context<Self>) -> Self::Result {
        use db::models::LoginState as DBLoginState;
        Ok(match DB.get_login(&msg.session, self.login_max_age)? {
            Some(v) => match v.state {
                DBLoginState::Complete => LoginState::LoggedIn,
                DBLoginState::Missing2Fa => LoginState::RequiresTOTP,
                DBLoginState::Requires2FaSetup => {
                    LoginState::RequiresTOTPSetup(DB.get_user(v.id)?.totp.into())
                }
            },
            None => LoginState::NotLoggedIn,
        })
    }
}

impl Handler<GetServicePerm> for UserService {
    type Result = Result<ServicePerm>;

    fn handle(&mut self, msg: GetServicePerm, _ctx: &mut Context<Self>) -> Self::Result {
        let uid = match DB.get_login(&msg.session, self.login_max_age)? {
            Some(v) => {
                use db::models::LoginState as DBLoginState;
                if v.state != DBLoginState::Complete {
                    return Err(UserError::InvalidPermissions);
                }
                v.id
            }
            None => return Err(UserError::InvalidSession),
        };
        Ok(DB.get_perm_service(uid, msg.service)?)
    }
}

impl Handler<LogoutUser> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, msg: LogoutUser, _ctx: &mut Context<Self>) -> Self::Result {
        DB.set_login(&msg.session, None)?;

        warn!("Not handling websocket!");
        //TODO: kick from websocket
        Ok(())
    }
}

impl Handler<CreateUser> for UserService {
    type Result = Result<CreateUserState>;

    fn handle(&mut self, msg: CreateUser, _ctx: &mut Context<Self>) -> Self::Result {
        if !self.is_admin(msg.invoker)? {
            return Err(UserError::InvalidPermissions);
        }
        self.create_user_unchecked(msg.user)
    }
}

impl Handler<SetPasswordCost> for UserService {
    type Result = ();

    fn handle(&mut self, msg: SetPasswordCost, _ctx: &mut Context<Self>) {
        self.brcypt_cost = msg.cost;
    }
}

impl Handler<EditUser> for UserService {
    type Result = Result<bool>;

    fn handle(&mut self, msg: EditUser, _ctx: &mut Context<Self>) -> Self::Result {
        if msg.invoker != msg.user_uid {
            if !self.is_admin(msg.invoker)? {
                return Ok(false);
            }
        }
        if let EditUserData::ServicePermission((service, perm)) = msg.data {
            DB.set_perm_service(msg.user_uid, service, perm)?;
        } else {
            let mut user = DB.get_user(msg.user_uid)?;
            match msg.data {
                EditUserData::Name(name) => user.name = name,
                EditUserData::Mail(email) => user.email = email,
                EditUserData::Password(pw) => {
                    user.password = bcrypt_password(&pw, self.brcypt_cost)?
                }
                // EditUserData::TOTP(secret) => user.totp_secret = secret,
                EditUserData::ServicePermission(_) => unreachable!(),
            }
            DB.update_user(user)?;
        }
        Ok(true)
    }
}

impl SystemService for UserService {}
impl Supervised for UserService {}
impl Actor for UserService {
    type Context = Context<Self>;

    fn started(&mut self, context: &mut Context<Self>) {
        IntervalFunc::new(
            std::time::Duration::from_secs(CLEANUP_INTERVAL),
            Self::cleanup_sessions,
        )
        .finish()
        .spawn(context);
    }
}
