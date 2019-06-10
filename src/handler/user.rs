use super::messages::*;

use crate::db;
use crate::db::{bcrypt_verify, models::ActiveLogin, DBInterface, DB};
use crate::web::models::{LoginState, UID};
use actix::prelude::*;
use bcrypt::BcryptError;
use failure::Fallible;
use metrohash::MetroHashMap;
use std::collections::HashSet;
/// Service for handling user related things
pub struct UserService {
    login_incomplete: MetroHashMap<String, UID>,
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

impl UserService {
    // fn get_user<'a>(&'a mut self, id: UID) -> Fallible<&'a User> {
    //     if let Some(v) = self.active_users.get(&id){
    //         return Ok(v);
    //     }
    //     let db_user = DB.get_user(id)?;
    //     let user =
    //     self.active_users.put(id,user);
    //     Ok(user)
    // }
}

impl Default for UserService {
    fn default() -> Self {
        Self {
            login_incomplete: MetroHashMap::default(),
        }
    }
}

/// Active user in system
// pub struct User {
//     pub id: UID,
//     pub name: String,
//     pub password: String,
//     pub totp_secret: Option<String>,
//     pub permissions: HashSet<String>,
// }

impl Handler<LoginUser> for UserService {
    type Result = Result<LoginState, Error>;

    fn handle(&mut self, msg: LoginUser, ctx: &mut Context<Self>) -> Self::Result {
        self.login_incomplete.remove(&msg.session);
        let uid = match DB.get_id_by_name(&msg.name)? {
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
                Ok(LoginState::SETUP_TOTP)
            }
        } else {
            DB.set_login(&msg.session, None)?;
            Ok(LoginState::Failed)
        }
    }
}

impl Handler<LogoutUser> for UserService {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: LogoutUser, ctx: &mut Context<Self>) -> Self::Result {
        DB.set_login(&msg.session, None)?;

        warn!("Not handling websocket DC!");
        //TODO: kick from websocket
        Ok(())
    }
}

impl SystemService for UserService {}
impl Supervised for UserService {}
impl Actor for UserService {
    type Context = Context<Self>;
}
