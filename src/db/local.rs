use super::models::*;
use super::Result;
use crate::crypto;
use bincode::{deserialize, serialize};
use std::time::SystemTime;

use failure;
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

macro_rules! ser {
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
    /// UID<->FullUser
    pub const USER: &'static str = "USER";
    /// email String<->UID
    pub const REL_MAIL_UID: &'static str = "REL_MAIL_UID";
    /// Specific see meta
    pub const META: &'static str = "META";
    /// "UID_SID"<->ServicePerm
    pub const PERMISSION_SERVICE: &'static str = "PERMISSIONS_SERVICE";
    /// "UID"<->ManPerm
    pub const PERMISSION_MANAGEMENT: &'static str = "PERMISSIONS_MANAGEMENT";
    /// session String<->UID
    pub const LOGINS: &'static str = "LOGINS";
    /// session String<->u64 time
    pub const REL_LOGIN_SEEN: &'static str = "REL_LOGIN_SEEN";
}

mod meta {
    /// UID - atomic counter for unique UID generation
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
    /// Generate Service-Perm ID
    #[inline]
    fn service_perm_key(uid: UID, sid: SID) -> Vec<u8> {
        serialize(&(uid, sid)).unwrap()
    }
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
        for _ in 0..max {
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
    fn create_user_inner(&self, new_user: NewUserEncrypted, id: UID) -> Result<FullUser> {
        let user = FullUser {
            id,
            email: new_user.email,
            verified: false,
            name: new_user.name,
            password: new_user.password_enc,
            totp: crypto::totp_gen_secret(),
            totp_complete: false,
        };
        self.open_tree(tree::USER)?.set(ser!(id), ser!(user))?;
        self.open_tree(tree::REL_MAIL_UID)?
            .set(ser!(user.email), ser!(id))?;
        Ok(user)
    }
}

impl super::DBInterface for DB {
    fn get_root_id(&self) -> UID {
        MIN_UID
    }

    fn create_user(&self, new_user: NewUserEncrypted) -> Result<FullUser> {
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

    fn get_perm_service(&self, id: UID, service: SID) -> Result<ServicePerm> {
        let v = self
            .open_tree(tree::PERMISSION_SERVICE)?
            .get(&DB::service_perm_key(id, service))?;
        Ok(match v {
            Some(v) => deserialize(&v)?,
            None => ServicePerm::default(),
        })
    }
    fn set_perm_service(&self, id: UID, service: SID, newPerms: ServicePerm) -> Result<()> {
        let tree = self.open_tree(tree::PERMISSION_SERVICE)?;
        let key = DB::service_perm_key(id, service);
        if newPerms.is_empty() {
            tree.del(&key)?;
        } else {
            self.open_tree(tree::PERMISSION_SERVICE)?
                .set(ser!(id), ser!(newPerms))?;
        }
        Ok(())
    }

    fn get_perm_man(&self, id: UID) -> Result<ManagementPerm> {
        unimplemented!()
    }
    fn set_perm_man(&self, id: UID, perm: &ManagementPerm) -> Result<()> {
        unimplemented!()
    }

    fn get_login(&self, session: &str, max_age: u32) -> Result<Option<ActiveLogin>> {
        Ok(match self.open_tree(tree::LOGINS)?.get(ser!(session))? {
            Some(v) => {
                let login = deserialize(&v)?;
                let outdated = match self.open_tree(tree::REL_LOGIN_SEEN)?.get(ser!(session))? {
                    Some(age_raw) => {
                        let age: u64 = deserialize(&age_raw)?;
                        super::get_current_time() - age > max_age as u64
                    }
                    None => {
                        warn!("Found inconsisent login without time!");
                        self.set_login(session, None)?;
                        false
                    }
                };

                if outdated {
                    None
                } else {
                    Some(login)
                }
            }
            None => None,
        })
    }
    fn set_login(&self, session: &str, state: Option<ActiveLogin>) -> Result<()> {
        let tree = self.open_tree(tree::LOGINS)?;
        match state {
            None => {
                tree.del(ser!(session))?;
                self.open_tree(tree::REL_LOGIN_SEEN)?.del(ser!(session))?;
            }
            Some(state) => {
                tree.set(ser!(session), ser!(state))?;
                self.update_login(session)?;
            }
        }
        Ok(())
    }

    fn update_login(&self, session: &str) -> Result<()> {
        self.open_tree(tree::REL_LOGIN_SEEN)?
            .set(ser!(session), ser!(super::get_current_time()))?;
        Ok(())
    }

    fn delete_old_logins(&self, max_age: u32) -> Result<usize> {
        let mut deleted = 0;
        let tree = self.open_tree(tree::REL_LOGIN_SEEN)?;
        for val in tree.iter() {
            let (session, time) = val?;
            let time: u64 = deserialize(&time)?;
            if super::get_current_time() - time > max_age as u64 {
                tree.del(session)?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    fn update_user(&self, user: FullUser) -> Result<()> {
        let old_email = self.get_user(user.id)?.email;
        self.open_tree(tree::USER)?.set(ser!(user.id), ser!(user))?;
        let tree = self.open_tree(tree::REL_MAIL_UID)?;
        if old_email != user.email {
            tree.del(ser!(old_email))?;
            tree.set(ser!(user.email), ser!(user.id))?;
        }
        Ok(())
    }

    fn delete_user(&self, id: UID) -> Result<()> {
        let user: FullUser = match self.open_tree(tree::USER)?.del(ser!(id))? {
            Some(u) => deserialize(&u)?,
            None => return Err(super::Error::InvalidUser(id).into()),
        };
        self.open_tree(tree::REL_MAIL_UID)?.del(ser!(user.email))?;
        self.open_tree(tree::PERMISSION_SERVICE)?.del(ser!(id))?;
        let sessions = self.open_tree(tree::LOGINS)?;
        for val in sessions.iter() {
            let (key, val) = val?;
            let al: ActiveLogin = deserialize(&val)?;
            if al.id == id {
                sessions.del(key)?;
            }
        }
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
        let data = self.open_tree(tree::REL_MAIL_UID)?.get(ser!(email))?;

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
