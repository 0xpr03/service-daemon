use super::models::*;
use super::{Error, Result};
use bincode::{deserialize, serialize};

use failure;
use failure::Fallible;
use sled::*;
use std::sync::Arc;

#[derive(Fail, Debug)]
pub enum DBError {
    #[fail(display = "Failed to open tree {}: {}", _1, _0)]
    TreeOpenFailed(#[cause] sled::Error, &'static str),
    #[fail(display = "Failed after retrying {} times", _0)]
    TooManyRetries(usize),
    #[fail(display = "Internal failure with DB err {}", _0)]
    SledError(#[cause] failure::Error),
    #[fail(display = "Interal failure with invalid Data {}", _0)]
    BincodeError(#[cause] Box<bincode::ErrorKind>),
    #[fail(display = "Internal failure to apply bcrypt to password! {}", _0)]
    EncryptioNError(#[cause] bcrypt::BcryptError),
}

impl From<Box<bincode::ErrorKind>> for DBError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        DBError::BincodeError(error)
    }
}

impl From<sled::Error> for DBError {
    fn from(error: sled::Error) -> Self {
        DBError::SledError(error.into())
    }
}
// fix for indirection creating misleading compiler errors
impl From<sled::Error> for super::Error {
    fn from(error: sled::Error) -> Self {
        super::Error::InternalError(error.into())
    }
}
impl From<Box<bincode::ErrorKind>> for super::Error {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        super::Error::InternalError(error.into())
    }
}

// impl From<pagecache::Error> for DBError {
//     fn from(error: pagecache::Error) -> Self {
//         DBError::SledError2(error)
//     }
// }

macro_rules! ser {
    ($expression:expr) => {
        serialize(&$expression).unwrap()
    };
}
macro_rules! serr {
    ($expression:expr) => {
        serialize(&$expression).unwrap()
    };
}

const MIN_UID: UID = 1;
const RESERVED: UID = MIN_UID - 1;
// const RES_V = RESERVED.to_le();
lazy_static! {
    static ref RES_V: Vec<u8> = ser!(RESERVED);
}

mod tree {
    pub const USER: &'static str = "USER";
    pub const REL_MAIL_UID: &'static str = "REL_MAIL_UID";
    pub const META: &'static str = "META";
    pub const PERMISSION: &'static str = "PERMISSIONS";
    pub const LOGINS: &'static str = "LOGINS";
    pub const REL_LOGIN_SEEN: &'static str = "REL_LOGIN_SEEN";
}

mod meta {
    pub const USER_AUTO_ID: &'static str = "USER_AUTO_ID";
}

#[derive(Clone)]
pub struct DB {
    db: Db,
}

impl Default for DB {
    fn default() -> Self {
        Self {
            db: Db::start_default("db.sled").unwrap(),
        }
    }
}

type WTree = Arc<Tree>;

impl DB {
    /// Open tree with wrapped error
    fn open_tree(&self, tree: &'static str) -> Result<WTree> {
        Ok(self
            .db
            .open_tree(tree)
            .map_err(|e| DBError::TreeOpenFailed(e, tree))?)
    }
    /// Generate new instance ID
    fn gen_user_id(&self) -> Result<UID> {
        let old =
            self.open_tree(tree::META)?
                .fetch_and_update(meta::USER_AUTO_ID, |v| match v {
                    // Basically fetch_add in sled style
                    Some(v) => Some(ser!(deserialize::<UID>(&v).unwrap() + 1)),
                    None => Some(ser!(MIN_UID + 1)),
                })?;

        Ok(match old {
            Some(v) => deserialize::<UID>(&v).unwrap(),
            None => MIN_UID,
        })
    }

    /// Generate ID and check that it's not in use currently.  
    /// target_db is the DB to check against
    fn gen_user_id_secure(&self) -> Result<UID> {
        let max = 100;
        for i in 0..max {
            let id = self.gen_user_id()?;
            if self.is_valid_uid(&id)? {
                warn!("Generated user ID exists already! {}", id);
                continue;
            } else {
                return Ok(id);
            }
        }
        error!("Out of UID generator retries!");
        Err(DBError::TooManyRetries(max).into())
    }
    /// Check if id is valid (taken)
    fn is_valid_uid(&self, id: &UID) -> Result<bool> {
        Ok(self.open_tree(tree::USER)?.contains_key(ser!(id))?)
    }
    /// Check if mail is taken
    fn is_mail_taken(&self, mail: &str) -> Result<bool> {
        Ok(self
            .open_tree(tree::REL_MAIL_UID)?
            .contains_key(ser!(mail))?)
    }
    /// Inner function to simulate transaction
    fn create_user_inner(&self, new_user: NewUser, id: UID) -> Result<FullUser> {
        let user = FullUser {
            id,
            email: new_user.email,
            name: new_user.name,
            password: super::bcrypt_password(&new_user.password)
                .map_err(|e| DBError::EncryptioNError(e))?,
            totp_secret: None,
        };
        self.open_tree(tree::USER)?.set(ser!(id), ser!(user))?;
        self.open_tree(tree::REL_MAIL_UID)?
            .set(ser!(user.email), ser!(id))?;
        Ok(user)
    }
}

impl super::DBInterface for DB {
    fn create_user(&self, new_user: NewUser) -> Result<FullUser> {
        let mail_ser = ser!(new_user.email);
        let claimed = self
            .open_tree(tree::REL_MAIL_UID)?
            .fetch_and_update(&mail_ser, |v| {
                match v {
                    None => Some(RES_V.to_vec()), // lock
                    Some(v) => Some(v.to_vec()),
                }
            })?;
        if claimed.is_some() {
            return Err(super::Error::EMailExists(new_user.email));
        }
        // first get ID, otherwise release lock
        let id = match self.gen_user_id_secure() {
            Ok(id) => id,
            Err(e) => {
                // make sure to clean the mail lock
                self.open_tree(tree::REL_MAIL_UID)?.del(mail_ser)?;
                return Err(e);
            }
        };
        // then create rest, otherwise cleanup
        match self.create_user_inner(new_user, id) {
            Ok(user) => Ok(user),
            Err(e) => {
                // make sure to clean the mail lock
                self.open_tree(tree::REL_MAIL_UID)?.del(mail_ser)?;
                // erase user entry
                self.open_tree(tree::USER)?.del(ser!(id))?;
                return Err(e);
            }
        }
    }

    fn get_users(&self) -> Result<Vec<MinUser>> {
        let mut users = Vec::new();
        for u in self.open_tree(tree::USER)?.iter() {
            let (_, v) = u?;
            let user: FullUser = deserialize(&v)?;
            users.push(MinUser {
                name: user.name,
                id: user.id,
                email: user.email,
            });
        }
        Ok(users)
    }

    fn get_user_permissions(&self, id: UID) -> Result<Vec<String>> {
        let v = self.open_tree(tree::PERMISSION)?.get(ser!(id))?;
        Ok(match v {
            Some(v) => deserialize(&v)?,
            None => Vec::new(),
        })
    }
    fn update_user_permission(&self, id: UID, perms: Vec<String>) -> Result<()> {
        self.open_tree(tree::PERMISSION)?
            .set(ser!(id), ser!(perms))?;
        Ok(())
    }

    fn get_login(&self, login: &str) -> Result<Option<ActiveLogin>> {
        Ok(match self.open_tree(tree::LOGINS)?.get(serr!(login))? {
            Some(v) => Some(deserialize(&v)?),
            None => None,
        })
    }
    fn set_login(&self, login: &str, state: Option<ActiveLogin>) -> Result<()> {
        let tree = self.open_tree(tree::LOGINS)?;
        match state {
            None => {
                tree.del(serr!(login))?;
                self.open_tree(tree::REL_LOGIN_SEEN)?.del(serr!(login))?;
            }
            Some(state) => {
                tree.set(serr!(login), ser!(state))?;
                self.update_login(login)?;
            }
        }
        Ok(())
    }

    fn update_login(&self, login: &str) -> Result<()> {
        self.open_tree(tree::REL_LOGIN_SEEN)?
            .set(serr!(login), ser!(super::get_current_time()))?;
        Ok(())
    }

    fn update_user(&self, user: FullUser) -> Result<()> {
        let old_email = self.get_user(user.id)?.email;
        self.open_tree(tree::USER)?.set(ser!(user.id), ser!(user))?;
        let tree = self.open_tree(tree::REL_MAIL_UID)?;
        tree.set(ser!(user.email), ser!(user.id))?;
        tree.del(ser!(old_email))?;
        Ok(())
    }

    fn delete_user(&self, id: UID) -> Result<()> {
        let user: FullUser = match self.open_tree(tree::USER)?.del(ser!(id))? {
            Some(u) => deserialize(&u)?,
            None => return Err(super::Error::InvalidUser(id).into()),
        };
        self.open_tree(tree::REL_MAIL_UID)?.del(ser!(user.email))?;
        self.open_tree(tree::PERMISSION)?.del(ser!(id))?;
        Ok(())
    }

    fn get_user(&self, id: UID) -> Result<FullUser> {
        let v = self
            .open_tree(tree::USER)?
            .get(ser!(id))?
            .ok_or(super::Error::InvalidUser(id))?;
        Ok(deserialize(&v)?)
    }

    fn get_id_by_email(&self, email: &str) -> Result<Option<UID>> {
        let data = self.open_tree(tree::REL_MAIL_UID)?.get(serr!(email))?;

        if let Some(v) = data {
            let id = deserialize(&v)?;
            // claimed, no data currently
            if id >= MIN_UID {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }
}
