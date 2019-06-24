use super::messages::*;

use super::error::UserError;
use crate::crypto::*;
use crate::db;
use crate::db::{
    models::{ActiveLogin, FullUser},
    DBInterface, DB,
};
use crate::web::models::{CreateUserState, LoginState, NewUser, NewUserEncrypted, UID};
use actix::prelude::*;
use metrohash::MetroHashMap;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::iter;

const ROOT_NAME: &'static str = "Root";
const ROOT_EMAIL: &'static str = "root@localhost";
const ROOT_PASSWORD_LENGTH: usize = 20;
const CLEANUP_INTERVAL: u64 = 60 * 20; // seconds

lazy_static! {
    static ref PERM_ROOT: String = String::from("ROOT");
    static ref PERM_CREATE_USER: String = String::from("CREATE_USERS");
    static ref PERM_DELETE_USER: String = String::from("DELETE_USERS");
    static ref PERM_EDIT_TOTP: String = String::from("EDIT_USERS_TOTP");
    static ref PERM_EDIT_NAME: String = String::from("EDIT_USERS_NAME");
    static ref PERM_EDIT_EMAIL: String = String::from("EDIT_USERS_EMAIL");
    static ref PERM_EDIT_PASSWORD: String = String::from("EDIT_USERS_PASSWORD");
}

/// Service for handling user related things
pub struct UserService {
    brcypt_cost: u32,
    login_incomplete: MetroHashMap<String, UID>,
}

impl UserService {
    fn cleanup_sessions(&mut self, context: &mut Context<Self>) {
        trace!("TODO: Cleanup user sessions");
    }
    fn has_permission(&self, uid: UID, perm: &String) -> Result<bool, UserError> {
        // &String due to https://github.com/rust-lang/rust/issues/42671
        let perms = DB.get_user_permissions(uid)?;
        if perms.contains(perm) {
            return Ok(true);
        }
        Ok(perms.contains(&PERM_ROOT))
    }
    fn get_root_user(&self) -> Result<Option<FullUser>, UserError> {
        match DB.get_user(DB.get_root_id()) {
            Ok(user) => Ok(Some(user)),
            Err(db::Error::InvalidUser(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    fn create_user_unchecked(&self, user: NewUser) -> Result<CreateUserState, UserError> {
        let user_enc = NewUserEncrypted {
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
            login_incomplete: MetroHashMap::default(),
        }
    }
}

impl Handler<StartupCheck> for UserService {
    type Result = Result<(), UserError>;

    fn handle(&mut self, _msg: StartupCheck, _ctx: &mut Context<Self>) -> Result<(), UserError> {
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

impl Handler<LoginUser> for UserService {
    type Result = Result<LoginState, UserError>;

    fn handle(&mut self, msg: LoginUser, _ctx: &mut Context<Self>) -> Self::Result {
        self.login_incomplete.remove(&msg.session);
        let uid = match DB.get_id_by_email(&msg.email)? {
            Some(v) => v,
            None => return Ok(LoginState::NotLoggedIn),
        };
        let user = DB.get_user(uid)?;
        if bcrypt_verify(&msg.password, &user.password)? {
            if user.totp_complete {
                self.login_incomplete.insert(msg.session, uid);
                Ok(LoginState::Requires_TOTP)
            } else {
                DB.set_login(
                    &msg.session,
                    Some(ActiveLogin {
                        state: db::models::LoginState::Requires_2FA_Setup,
                        id: uid,
                    }),
                )?;
                Ok(LoginState::Requires_TOTP_Setup(user.totp.into()))
            }
        } else {
            DB.set_login(&msg.session, None)?;
            Ok(LoginState::NotLoggedIn)
        }
    }
}

impl Handler<CheckSession> for UserService {
    type Result = Result<LoginState, UserError>;

    fn handle(&mut self, msg: CheckSession, _ctx: &mut Context<Self>) -> Self::Result {
        use db::models::LoginState as DBLoginState;
        Ok(match DB.get_login(&msg.session)? {
            Some(v) => match v.state {
                DBLoginState::Complete => LoginState::LoggedIn,
                DBLoginState::Missing_2FA => LoginState::Requires_TOTP,
                DBLoginState::Requires_2FA_Setup => {
                    LoginState::Requires_TOTP_Setup(DB.get_user(v.id)?.totp.into())
                }
            },
            None => LoginState::NotLoggedIn,
        })
    }
}

impl Handler<LogoutUser> for UserService {
    type Result = Result<(), UserError>;

    fn handle(&mut self, msg: LogoutUser, _ctx: &mut Context<Self>) -> Self::Result {
        DB.set_login(&msg.session, None)?;

        warn!("Not handling websocket DC!");
        //TODO: kick from websocket
        Ok(())
    }
}

impl Handler<CreateUser> for UserService {
    type Result = Result<CreateUserState, UserError>;

    fn handle(&mut self, msg: CreateUser, _ctx: &mut Context<Self>) -> Self::Result {
        if !self.has_permission(msg.invoker, &PERM_CREATE_USER)? {
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
    type Result = Result<bool, UserError>;

    fn handle(&mut self, msg: EditUser, _ctx: &mut Context<Self>) -> Self::Result {
        if msg.invoker != msg.user_uid {
            let required = match msg.data {
                EditUserData::Name(_) => self.has_permission(msg.invoker, &PERM_EDIT_NAME)?,
                EditUserData::Password(_) => {
                    self.has_permission(msg.invoker, &PERM_EDIT_PASSWORD)?
                }
                EditUserData::Permission(_) => self.has_permission(msg.invoker, &PERM_ROOT)?,
                // EditUserData::TOTP(_) => self.has_permission(msg.invoker, &PERM_EDIT_TOTP)?,
                EditUserData::Mail(_) => self.has_permission(msg.invoker, &PERM_EDIT_EMAIL)?,
            };
            if !required {
                return Ok(false);
            }
        }
        if let EditUserData::Permission(perm) = msg.data {
            if !self.has_permission(msg.invoker, &PERM_ROOT)? {
                return Ok(false);
            }
            DB.update_user_permission(msg.user_uid, perm)?;
        } else {
            let mut user = DB.get_user(msg.user_uid)?;
            match msg.data {
                EditUserData::Name(name) => user.name = name,
                EditUserData::Mail(email) => user.email = email,
                EditUserData::Password(pw) => {
                    user.password = bcrypt_password(&pw, self.brcypt_cost)?
                }
                // EditUserData::TOTP(secret) => user.totp_secret = secret,
                EditUserData::Permission(_) => unreachable!(),
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
