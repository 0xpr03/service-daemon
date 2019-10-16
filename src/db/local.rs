use super::models::*;
use super::Result;
use crate::crypto;
use bincode::{deserialize, serialize};
use std::collections::HashMap;

use failure;
use sled::*;

#[derive(Fail, Debug)]
pub enum DBError {
    #[fail(display = "Failed to open tree {}: {}", _1, _0)]
    TreeOpenFailed(#[cause] sled::Error, &'static str),
    #[fail(display = "Failed after retrying {} times", _0)]
    TooManyRetries(usize),
    #[fail(display = "Internal failure with DB err {}", _0)]
    SledError(#[cause] failure::Error),
    #[fail(display = "Error with transaction: {}", _0)]
    SledTransactionError(#[cause] sled::TransactionError),
    #[fail(display = "Interal failure with invalid Data {}", _0)]
    BincodeError(#[cause] Box<bincode::ErrorKind>),
}

impl From<Box<bincode::ErrorKind>> for DBError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        DBError::BincodeError(error)
    }
}

impl From<sled::TransactionError> for DBError {
    fn from(error: sled::TransactionError) -> Self {
        DBError::SledTransactionError(error.into())
    }
}

// fix for indirection creating misleading compiler errors
impl From<sled::TransactionError> for super::Error {
    fn from(error: sled::TransactionError) -> Self {
        super::Error::InternalError(error.into())
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
            // TODO: this does NOT return but panic when the DB is already in use
            db: match Db::open("db.sled") {
                Err(e) => {
                    error!("Unable to start local DB: {}", e);
                    panic!("Unable to start local DB: {}", e);
                }
                Ok(v) => v,
            },
        }
    }
}

impl DB {
    /// Serialize a multi-key for use in indexing, enforcing big endianess for sled
    #[inline]
    fn ser_key<T: ?Sized + serde::Serialize>(t: &T) -> Vec<u8> {
        bincode::config().big_endian().serialize(t).unwrap()
    }
    /// Deserialize a multi-key used in indexing, enforcing big endianess for sled
    #[inline]
    fn deser_key<'a, T: serde::Deserialize<'a>>(bytes: &'a [u8]) -> T {
        bincode::config().big_endian().deserialize::<T>(bytes).unwrap()
    }
    /// Generate serialized Service-Perm ID
    #[inline]
    fn service_perm_key(uid: UID, sid: SID) -> Vec<u8> {
        Self::ser_key(&(uid, sid))
    }
    #[inline]
    fn service_perm_key_reverse(data: &[u8]) -> Result<(UID, SID)> {
        Ok(bincode::config().big_endian().deserialize(data)?)
    }
    /// Open tree with wrapped error
    fn open_tree(&self, tree: &'static str) -> Result<Tree> {
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
}

impl super::DBInterface for DB {
    fn new_temp() -> Self {
        let config = ConfigBuilder::default().temporary(true);

        Self {
            db: Db::start(config.build()).unwrap(),
        }
    }

    fn get_root_id(&self) -> UID {
        MIN_UID
    }

    fn create_user(&self, new_user: NewUserEnc) -> Result<FullUser> {
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
            return Err(super::Error::EMailExists);
        }
        // first get ID, otherwise release lock
        let id = match self.gen_user_id_secure() {
            Ok(id) => id,
            Err(e) => {
                // make sure to clean the mail lock
                self.open_tree(tree::REL_MAIL_UID)?.remove(mail_ser)?;
                return Err(e);
            }
        };

        let user = FullUser {
            id,
            email: new_user.email,
            verified: false,
            name: new_user.name,
            password: new_user.password_enc,
            totp: crypto::totp_gen_secret(),
            totp_complete: false,
            admin: false,
        };
        let user_tree = self.open_tree(tree::USER)?;
        let rel_mail_uid_tree = self.open_tree(tree::REL_MAIL_UID)?;

        // use transaction, avoid dangling user entries
        (&user_tree, &rel_mail_uid_tree).transaction(|(user_tree, rel_mail_uid_tree)| {
            user_tree.insert(ser!(id), ser!(user))?;
            rel_mail_uid_tree.insert(ser!(user.email), ser!(id))?;
            Ok(())
        })?;
        Ok(user)
    }

    fn get_users(&self) -> Result<Vec<UserMin>> {
        let mut users = Vec::new();
        for u in self.open_tree(tree::USER)?.iter() {
            let (_, v) = u?;
            let user: FullUser = deserialize(&v)?;
            users.push(UserMin::from(user));
        }
        Ok(users)
    }

    fn get_perm_admin(&self) -> Result<Vec<UID>> {
        let mut vec = Vec::new();
        for val in self.open_tree(tree::USER)?.iter() {
            let (uid_r, user_r) = val?;
            if deserialize::<FullUser>(&user_r)?.admin {
                vec.push(deserialize(&uid_r)?);
            }
        }
        Ok(vec)
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

    fn get_all_perm_service(&self, id: UID) -> Result<HashMap<SID, ServicePerm>> {
        let v = self.open_tree(tree::PERMISSION_SERVICE)?;
        let iter = v.range(Self::service_perm_key(id, 0)..Self::service_perm_key(id+1,0));
        Ok(iter
            .map(|v| {
                let (key, value) = v?;
                let (_, sid) = DB::service_perm_key_reverse(&key)?;
                Ok((sid, deserialize::<ServicePerm>(&value)?))
            })
            .collect::<Result<HashMap<SID, ServicePerm>>>()?)
    }

    fn set_perm_service(&self, id: UID, service: SID, new_perms: ServicePerm) -> Result<()> {
        let tree = self.open_tree(tree::PERMISSION_SERVICE)?;
        let key = DB::service_perm_key(id, service);
        if new_perms.is_empty() {
            tree.remove(&key)?;
        } else {
            tree.insert(&key, ser!(new_perms))?;
        }
        Ok(())
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
                tree.remove(ser!(session))?;
                self.open_tree(tree::REL_LOGIN_SEEN)?
                    .remove(ser!(session))?;
            }
            Some(state) => {
                tree.insert(ser!(session), ser!(state))?;
                self.update_login(session)?;
            }
        }
        Ok(())
    }

    fn update_login(&self, session: &str) -> Result<()> {
        self.open_tree(tree::REL_LOGIN_SEEN)?
            .insert(ser!(session), ser!(super::get_current_time()))?;
        Ok(())
    }

    fn delete_old_logins(&self, max_age: u32) -> Result<usize> {
        let mut deleted = 0;
        let tree_rel = self.open_tree(tree::REL_LOGIN_SEEN)?;
        let tree_logins = self.open_tree(tree::LOGINS)?;
        for val in tree_rel.iter() {
            let (session, time) = val?;
            let time: u64 = deserialize(&time)?;
            if super::get_current_time() - time > max_age as u64 {
                tree_rel.remove(&session)?;
                tree_logins.remove(session)?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    fn update_user(&self, user: FullUser) -> Result<()> {
        if !self.is_valid_uid(&user.id)? {
            return Err(super::Error::InvalidUser(user.id).into());
        }
        let old_email = self.get_user(user.id)?.email;
        if old_email != user.email {
            debug!("old mail != new mail");
            let tree = self.open_tree(tree::REL_MAIL_UID)?;
            match tree.cas(ser!(user.email), None as Option<&[u8]>, Some(ser!(user.id)))? {
                Err(_) => {
                    return Err(super::Error::EMailExists);
                }
                Ok(_) => {
                    debug!("{} not in use", user.email);
                    tree.remove(ser!(old_email))?;
                }
            }
        }
        self.open_tree(tree::USER)?
            .insert(ser!(user.id), ser!(user))?;

        Ok(())
    }

    fn delete_user(&self, id: UID) -> Result<()> {
        let user: FullUser = match self.open_tree(tree::USER)?.remove(ser!(id))? {
            Some(u) => deserialize(&u)?,
            None => return Err(super::Error::InvalidUser(id).into()),
        };
        self.open_tree(tree::REL_MAIL_UID)?
            .remove(ser!(user.email))?;
        self.open_tree(tree::PERMISSION_SERVICE)?.remove(ser!(id))?;
        let sessions = self.open_tree(tree::LOGINS)?;
        for val in sessions.iter() {
            let (key, val) = val?;
            let al: ActiveLogin = deserialize(&val)?;
            if al.id == id {
                sessions.remove(key)?;
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::models::*;
    use crate::db::DBInterface;
    type Pair = (i32, i32);
    use tempfile::tempdir;
    use std::collections::HashMap;

    /// Assert we're doing the endian-ness right
    #[test]
    fn test_service_perm() {
        let tmp_dir = tempdir().unwrap();
        let db = DB{
            db: match Db::open(format!("{}/db", tmp_dir.path().to_string_lossy())) {
                Err(e) => {
                    error!("Unable to start local DB: {}", e);
                    panic!("Unable to start local DB: {}", e);
                }
                Ok(v) => v,
            }
        };

        db.set_perm_service(1, 256, ServicePerm::from_bits_truncate(1)).unwrap();
        db.set_perm_service(1, 258, ServicePerm::from_bits_truncate(9)).unwrap();
        db.set_perm_service(2, 257, ServicePerm::from_bits_truncate(2)).unwrap();
        db.set_perm_service(256, 1, ServicePerm::from_bits_truncate(4)).unwrap();
        db.set_perm_service(257, 2, ServicePerm::from_bits_truncate(8)).unwrap();

        let mut map = HashMap::new();
        map.insert(258, ServicePerm::from_bits_truncate(9));
        map.insert(256, ServicePerm::from_bits_truncate(1));
        assert_eq!(db.get_all_perm_service(1).unwrap(),map);
    }

    /// Assert that our serialization works as intended
    /// 
    /// This also revealed that we need to enforce big endian for this to work
    #[test]
    #[ignore]
    fn test_range_service_perm() {
        let config = ConfigBuilder::default().temporary(true);

        let mut cfg = bincode::config();
        cfg.big_endian();

        let db = Db::start(config.build()).unwrap();
        // take values that require > 1 byte
        db.insert(cfg.serialize(&(1, 2)).unwrap(), cfg.serialize(&format!("{}-{}", 1, 2)).unwrap()).unwrap();
        db.insert(cfg.serialize(&(2, 1)).unwrap(), cfg.serialize(&format!("{}-{}", 2, 1)).unwrap()).unwrap();
        db.insert(cfg.serialize(&(512, 420)).unwrap(), cfg.serialize(&format!("{}-{}", 512, 420)).unwrap()).unwrap();
        db.insert(cfg.serialize(&(420, 512)).unwrap(), cfg.serialize(&format!("{}-{}", 420, 512)).unwrap()).unwrap();

        let iter = db.range(cfg.serialize(&(1,1)).unwrap()..cfg.serialize(&(513,513)).unwrap());
        let mut i = 0;
        for elem in iter {
            let (key, val) = elem.unwrap();
            let (a, b) = cfg.deserialize::<Pair>(&key).unwrap();
            println!("{} {}",a,b);
            assert_eq!(format!("{}-{}", a, b), cfg.deserialize::<String>(&val).unwrap());
            i += 1;
        }
        assert_eq!(4,i);

        // now test a sub-range
        db.insert(cfg.serialize(&(420, 510)).unwrap(), cfg.serialize(&format!("{}-{}", 420, 510)).unwrap()).unwrap();
        db.insert(cfg.serialize(&(420, 509)).unwrap(), cfg.serialize(&format!("{}-{}", 420, 509)).unwrap()).unwrap();
        db.insert(cfg.serialize(&(420, 508)).unwrap(), cfg.serialize(&format!("{}-{}", 420, 508)).unwrap()).unwrap();

        let iter = db.range(cfg.serialize(&(420,508)).unwrap()..cfg.serialize(&(420,510)).unwrap());
        let mut i = 0;
        for elem in iter {
            let (key, val) = elem.unwrap();
            let (a, b) = cfg.deserialize::<Pair>(&key).unwrap();
            println!("{} {}",a,b);
            assert_eq!(format!("{}-{}", a, b), cfg.deserialize::<String>(&val).unwrap());
            i += 1;
        }
        assert_eq!(2,i);
    }
}
