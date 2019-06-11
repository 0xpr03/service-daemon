use super::messages::*;

use crate::db;
use crate::db::{bcrypt_verify, models::ActiveLogin, DBInterface, DB};
use crate::web::models::{CreateUserState, LoginState, UID};
use actix::prelude::*;
use bcrypt::BcryptError;
use metrohash::MetroHashMap;

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
    login_incomplete: MetroHashMap<String, UID>,
}

impl UserService {
    fn has_permission(&self, uid: UID, perm: &String) -> Result<bool, Error> {
        // &String due to https://github.com/rust-lang/rust/issues/42671
        let perms = DB.get_user_permissions(uid)?;
        if perms.contains(perm) {
            return Ok(true);
        }
        Ok(perms.contains(&PERM_ROOT))
    }
}

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "Internal DB error {}", _0)]
    DBError(db::Error),
    #[fail(display = "Error with password hashing! {}", _0)]
    HashError(#[cause] BcryptError),
    #[fail(display = "Lacking permissions!")]
    InvalidPermissions,
}

impl From<bcrypt::BcryptError> for Error {
    fn from(error: bcrypt::BcryptError) -> Self {
        Error::HashError(error)
    }
}
impl From<db::Error> for Error {
    fn from(error: db::Error) -> Self {
        Error::DBError(error)
    }
}

impl Default for UserService {
    fn default() -> Self {
        Self {
            login_incomplete: MetroHashMap::default(),
        }
    }
}

impl Handler<LoginUser> for UserService {
    type Result = Result<LoginState, Error>;

    fn handle(&mut self, msg: LoginUser, _ctx: &mut Context<Self>) -> Self::Result {
        self.login_incomplete.remove(&msg.session);
        let uid = match DB.get_id_by_email(&msg.email)? {
            Some(v) => v,
            None => return Ok(LoginState::Failed),
        };
        let user = DB.get_user(uid)?;
        if bcrypt_verify(&msg.password, &user.password)? {
            if user.totp_secret.is_some() {
                self.login_incomplete.insert(msg.session, uid);
                Ok(LoginState::TOTP)
            } else {
                DB.set_login(
                    &msg.session,
                    Some(ActiveLogin {
                        incomplete: true,
                        id: uid,
                    }),
                )?;
                Ok(LoginState::SetupTOTP)
            }
        } else {
            DB.set_login(&msg.session, None)?;
            Ok(LoginState::Failed)
        }
    }
}

impl Handler<LogoutUser> for UserService {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: LogoutUser, _ctx: &mut Context<Self>) -> Self::Result {
        DB.set_login(&msg.session, None)?;

        warn!("Not handling websocket DC!");
        //TODO: kick from websocket
        Ok(())
    }
}

impl Handler<CreateUser> for UserService {
    type Result = Result<CreateUserState, Error>;

    fn handle(&mut self, msg: CreateUser, _ctx: &mut Context<Self>) -> Self::Result {
        if !self.has_permission(msg.invoker, &PERM_CREATE_USER)? {
            return Err(Error::InvalidPermissions);
        }
        let v = DB.create_user(msg.user);
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

impl Handler<EditUser> for UserService {
    type Result = Result<bool, Error>;

    fn handle(&mut self, msg: EditUser, _ctx: &mut Context<Self>) -> Self::Result {
        if msg.invoker != msg.user_uid {
            let required = match msg.data {
                EditUserData::Name(_) => self.has_permission(msg.invoker, &PERM_EDIT_NAME)?,
                EditUserData::Password(_) => {
                    self.has_permission(msg.invoker, &PERM_EDIT_PASSWORD)?
                }
                EditUserData::Permission(_) => self.has_permission(msg.invoker, &PERM_ROOT)?,
                EditUserData::TOTP(_) => self.has_permission(msg.invoker, &PERM_EDIT_TOTP)?,
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
                EditUserData::Password(pw) => user.password = db::bcrypt_password(&pw)?,
                EditUserData::TOTP(secret) => user.totp_secret = Some(secret),
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
}
