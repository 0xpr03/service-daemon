use super::models::*;
use super::Result;
use crate::crypto;
use bincode::Options;
use bincode::{deserialize, serialize};
use std::collections::HashMap;
use std::io::Write;

use failure;
use sled::*;

#[derive(Fail, Debug)]
pub enum DBError {
    #[fail(display = "Failed to open tree {}: {}", _1, _0)]
    TreeOpenFailed(#[cause] sled::Error, &'static str),
    #[fail(display = "Failed after retrying {} times", _0)]
    TooManyRetries(usize),
    #[fail(display = "Internal failure with DB err: {}", _0)]
    SledError(#[cause] failure::Error),
    // can't mark a #[cause] due to missing Fail impl on () for <()>
    #[fail(display = "Error with transaction: {:?}", _0)]
    SledTransactionError(Box<sled::TransactionError<()>>),
    #[fail(display = "Interal failure with invalid Data: {}", _0)]
    BincodeError(#[cause] Box<bincode::ErrorKind>),
    #[fail(display = "Failed to perform IO: {}", _0)]
    IO(#[cause] std::io::Error),
}

impl From<Box<bincode::ErrorKind>> for DBError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        DBError::BincodeError(error)
    }
}

impl From<sled::TransactionError<()>> for DBError {
    fn from(error: sled::TransactionError<()>) -> Self {
        DBError::SledTransactionError(Box::new(error))
    }
}

impl From<std::io::Error> for DBError {
    fn from(error: std::io::Error) -> Self {
        DBError::IO(error)
    }
}

// fix for indirection creating misleading compiler errors
impl From<sled::TransactionError<()>> for super::Error {
    fn from(error: sled::TransactionError<()>) -> Self {
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

impl From<std::io::Error> for super::Error {
    fn from(error: std::io::Error) -> Self {
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
    pub const USER: &str = "USER";
    /// email String<->UID
    pub const REL_MAIL_UID: &str = "REL_MAIL_UID";
    /// Specific see meta
    pub const META: &str = "META";
    /// "UID_SID"<->ServicePerm
    pub const PERMISSION_SERVICE: &str = "PERMISSIONS_SERVICE";
    /// session String<->UID
    pub const LOGINS: &str = "LOGINS";
    /// session String<->u64 time
    pub const REL_LOGIN_SEEN: &str = "REL_LOGIN_SEEN";
    /// service log entries (SID,Db::generate_id)->LogEntry
    pub const LOG_ENTRIES: &str = "LOG_ENTRIES";
    /// service log console snapshots (LogEntry additional data)
    /// (SID,Db::generate_id)->ConsoleOutput
    pub const LOG_CONSOLE: &str = "LOG_CONSOLE";
    pub const ALL: &[&str] = &[
        USER,
        REL_MAIL_UID,
        META,
        PERMISSION_SERVICE,
        LOGINS,
        REL_LOGIN_SEEN,
        LOG_ENTRIES,
        LOG_CONSOLE,
    ];
}

mod meta {
    /// UID - atomic counter for unique UID generation
    pub const USER_AUTO_ID: &str = "USER_AUTO_ID";
}

#[derive(Clone)]
pub struct DB {
    db: Db,
}

impl Default for DB {
    fn default() -> Self {
        Self {
            // TODO: this does NOT return but panic when the DB is already in use
            db: match open("db.sled") {
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
    ///
    /// Comes with some performance penalty, should therefore not be used everywhere
    #[inline]
    fn ser_key<T: ?Sized + serde::Serialize>(t: &T) -> Vec<u8> {
        bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .with_no_limit()
            .with_big_endian()
            .serialize(t)
            .unwrap()
    }
    /// Deserialize a multi-key used in indexing, enforcing big endianess for sled
    #[inline]
    fn deser_key<'a, T: serde::Deserialize<'a>>(bytes: &'a [u8]) -> T {
        Self::deser_key_opt::<T>(bytes).unwrap()
    }
    #[inline]
    fn deser_key_opt<'a, T: serde::Deserialize<'a>>(
        bytes: &'a [u8],
    ) -> std::result::Result<T, Box<bincode::ErrorKind>> {
        bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .with_no_limit()
            .with_big_endian()
            .deserialize::<T>(bytes)
    }
    /// Generate serialized Service-Perm ID
    #[inline]
    fn service_perm_key(uid: UID, sid: SID) -> Vec<u8> {
        Self::ser_key(&(uid, sid))
    }
    #[inline]
    fn service_perm_key_reverse(data: &[u8]) -> Result<(UID, SID)> {
        Ok(bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .with_no_limit()
            .with_big_endian()
            .deserialize(data)?)
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
            if self.is_valid_uid(id)? {
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
    fn is_valid_uid(&self, id: UID) -> Result<bool> {
        Ok(self.open_tree(tree::USER)?.contains_key(ser!(id))?)
    }

    /// Print DB Stats
    fn print_stats(&self) -> Result<()> {
        for name in self.db.tree_names().into_iter() {
            let tree = self.db.open_tree(&name)?;
            let name = std::str::from_utf8(&name).unwrap();
            println!("{} {}", name, tree.len());
        }
        Ok(())
    }
}

impl super::DBInterface for DB {
    fn new_temp() -> Self {
        let config = Config::default().temporary(true);

        Self {
            db: config.open().expect("Can't start local DB!"),
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
            totp_setup_complete: false,
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
        let iter = v.range(Self::service_perm_key(id, 0)..Self::service_perm_key(id + 1, 0));
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
                        super::get_current_time() - age > u64::from(max_age)
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
            if super::get_current_time() - time > u64::from(max_age) {
                tree_rel.remove(&session)?;
                tree_logins.remove(session)?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    fn update_user(&self, user: FullUser) -> Result<()> {
        if !self.is_valid_uid(user.id)? {
            return Err(super::Error::InvalidUser(user.id));
        }
        let old_email = self.get_user(user.id)?.email;
        if old_email != user.email {
            debug!("old mail != new mail");
            let tree = self.open_tree(tree::REL_MAIL_UID)?;
            match tree.compare_and_swap(
                ser!(user.email),
                None as Option<&[u8]>,
                Some(ser!(user.id)),
            )? {
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
            None => return Err(super::Error::InvalidUser(id)),
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

    fn insert_log_entry(
        &self,
        service: SID,
        entry: NewLogEntry,
        console: Option<ConsoleOutput>,
    ) -> Result<()> {
        let key = self.db.generate_id()?;
        let entry = LogEntry::new(key, entry, console.is_some());

        let log_entries_tree = self.open_tree(tree::LOG_ENTRIES)?;
        if let Some(console_data) = console {
            let log_console_tree = self.open_tree(tree::LOG_CONSOLE)?;
            (&log_entries_tree, &log_console_tree).transaction(
                move |(log_entries_tree, log_console_tree)| {
                    log_entries_tree.insert(Self::ser_key(&(service, key)), ser!(entry))?;
                    log_console_tree.insert(Self::ser_key(&(service, key)), ser!(console_data))?;
                    Ok(())
                },
            )?;
        } else {
            log_entries_tree.insert(Self::ser_key(&(service, key)), ser!(entry))?;
        }
        Ok(())
    }

    fn get_service_console_log(
        &self,
        service: SID,
        log_id: LogID,
    ) -> Result<Option<ConsoleOutput>> {
        if let Some(v) = self
            .open_tree(tree::LOG_CONSOLE)?
            .get(Self::ser_key(&(service, log_id)))?
        {
            return Ok(Some(deserialize(&v)?));
        }
        Ok(None)
    }

    fn get_service_log_details(
        &self,
        service: SID,
        log_id: LogID,
    ) -> Result<Option<LogEntryResolved>> {
        if let Some(v) = self
            .open_tree(tree::LOG_ENTRIES)?
            .get(Self::ser_key(&(service, log_id)))?
        {
            let entry: LogEntry = deserialize(&v)?;
            let invoker = match entry.invoker {
                None => None,
                Some(uid) => Some(Invoker::from(self.get_user(uid)?)),
            };

            let entry = LogEntryResolved {
                time: entry.time,
                action: entry.action,
                id: entry.log_id,
                invoker,
                console_log: entry.console_log,
            };
            return Ok(Some(entry));
        }
        Ok(None)
    }

    fn service_log_limited(&self, service: SID, limit: usize) -> Result<Vec<LogEntryResolved>> {
        let mut iter = self
            .open_tree(tree::LOG_ENTRIES)?
            .scan_prefix(Self::ser_key(&service));

        let mut invoker_map: HashMap<UID, Invoker> = HashMap::new();
        let mut entries = Vec::with_capacity(limit);
        while let Some(e) = iter.next_back() {
            let (_, v) = e?;
            let entry: LogEntry = deserialize(&v)?;

            // basically try_map to convert uid to Invoker if existing
            let invoker = match entry.invoker {
                None => None,
                Some(uid) => Some(match invoker_map.get(&uid) {
                    Some(e) => e.clone(),
                    None => {
                        let invoker = Invoker::from(self.get_user(uid)?);
                        invoker_map.insert(uid, invoker.clone());
                        invoker
                    }
                }),
            };

            let entry = LogEntryResolved {
                time: entry.time,
                action: entry.action,
                id: entry.log_id,
                invoker,
                console_log: entry.console_log,
            };
            entries.push(entry);
            if entries.len() >= limit {
                break;
            }
        }
        Ok(entries)
    }

    fn service_log_date(&self, service: SID, from: Date, to: Date) -> Result<Vec<LogEntry>> {
        // sadly a full table scan for a service, as the date isn't really the same always
        let mut entries = Vec::new();
        let tree = self.open_tree(tree::LOG_ENTRIES)?;
        for entry in tree.scan_prefix(Self::ser_key(&(service))) {
            let (_, v) = entry?;
            let entry: LogEntry = deserialize(&v)?;
            if from <= entry.time && to >= entry.time {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    fn service_log_minmax(&self, service: SID) -> Result<Option<(Date, Date)>> {
        let tree = self.open_tree(tree::LOG_ENTRIES)?;
        let mut iter = tree.scan_prefix(Self::ser_key(&service));
        if let Some(min) = iter.next() {
            let (k, _) = min?;
            let (_, min) = Self::deser_key::<(SID, Date)>(&k);
            return match iter.next_back() {
                Some(max) => {
                    let (k, _) = max?;
                    let (_, max) = Self::deser_key::<(SID, Date)>(&k);
                    Ok(Some((min, max)))
                }
                None => Ok(Some((min, min))),
            };
        }
        Ok(None)
    }

    fn cleanup(&self, max_age: Date) -> Result<()> {
        let log_tree = self.open_tree(tree::LOG_ENTRIES)?;

        let mut keys: Vec<_> = Vec::new();

        let mut items = 0;
        let mut invalid_val = 0;
        let mut invalid_key = 0;

        let cfg = bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .with_no_limit()
            .with_big_endian();
        {
            for r in log_tree.iter() {
                items += 1;
                let (k, v) = r.expect("Can't read next entry");
                //println!("Entry: {:?}",v);
                let entry: LogEntry = match cfg.deserialize(&v) {
                    Ok(v) => v,
                    Err(e) => {
                        invalid_val += 1;
                        error!("Can't deserialize value of key:{:?} {:?}!", k, e);
                        continue;
                    }
                };
                match Self::deser_key_opt::<(SID, LogID)>(&k) {
                    Ok((_s, _d)) => (),
                    Err(e) => {
                        invalid_key += 1;
                        let parsed = chrono::NaiveDateTime::from_timestamp(entry.time / 1000, 0)
                            .format("%Y-%m-%d %H:%M:%S");
                        error!(
                            "Key error {} \t {} \t {:?} {:?} {:?}",
                            entry.time, parsed, k, e, entry
                        );
                    }
                }

                if entry.time < max_age {
                    keys.push(k);
                    //let parsed = chrono::NaiveDateTime::from_timestamp(entry.time/1000,0).format("%Y-%m-%d %H:%M:%S");
                    //debug!("Found deletable entry from {}",parsed);
                }
            }
        }

        let console_tree = self.open_tree(tree::LOG_CONSOLE)?;
        info!("Found {} deletable entries.", keys.len());

        let mut batch = Batch::default();

        for v in keys {
            batch.remove(v);
        }

        info!("Applying batch removal");
        log_tree.apply_batch(batch.clone())?;
        console_tree.apply_batch(batch)?;

        //info!("Flushing DB");
        //log_tree.flush()?;
        //console_tree.flush()?;

        info!(
            "Found {} invalid val {} invalid key {}",
            items, invalid_val, invalid_key
        );
        self.print_stats()?;
        info!("Finished, press any key to exit. Waiting allows the GC to take place.");
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();

        Ok(())
    }

    fn export(&self, file: &str) -> Result<()> {
        let mut file = std::fs::File::create(file)?;
        let mut dmp: DBDump = DBDump::new();
        for (a, b, c) in self.db.export() {
            dmp.push((a, b, c.collect()));
        }
        info!("Found {} entries", dmp.len());
        bincode::options()
            .with_fixint_encoding()
            .with_no_limit()
            .with_big_endian()
            .serialize_into(&mut file, &dmp)?;
        file.flush()?;
        Ok(())
    }

    fn import(&self, file: &str) -> Result<()> {
        // create all required trees
        for v in tree::ALL {
            let _ = self.open_tree(v)?;
        }

        let file = std::fs::File::open(file)?;
        let dmp: DBDump = bincode::options()
            .with_fixint_encoding()
            .with_no_limit()
            .with_big_endian()
            .deserialize_from(file)?;
        info!("Found {} entries", dmp.len());
        let dmp: Vec<_> = dmp
            .into_iter()
            .map(|(a, b, c)| (a, b, c.into_iter()))
            .collect();

        self.db.import(dmp);
        self.db.flush()?;
        Ok(())
    }
}

type DBDump = Vec<(Vec<u8>, Vec<u8>, Vec<Vec<Vec<u8>>>)>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::DBInterface;
    type Pair = (i32, i32);
    use std::collections::HashMap;
    use tempfile::tempdir;

    /// Assert we're doing the endian-ness right
    #[test]
    fn test_service_perm() {
        let tmp_dir = tempdir().unwrap();
        let db = DB {
            db: match open(format!("{}/db", tmp_dir.path().to_string_lossy())) {
                Err(e) => {
                    error!("Unable to start local DB: {}", e);
                    panic!("Unable to start local DB: {}", e);
                }
                Ok(v) => v,
            },
        };

        db.set_perm_service(1, 256, ServicePerm::from_bits_truncate(1))
            .unwrap();
        db.set_perm_service(1, 258, ServicePerm::from_bits_truncate(9))
            .unwrap();
        db.set_perm_service(2, 257, ServicePerm::from_bits_truncate(2))
            .unwrap();
        db.set_perm_service(256, 1, ServicePerm::from_bits_truncate(4))
            .unwrap();
        db.set_perm_service(257, 2, ServicePerm::from_bits_truncate(8))
            .unwrap();

        let mut map = HashMap::new();
        map.insert(258, ServicePerm::from_bits_truncate(9));
        map.insert(256, ServicePerm::from_bits_truncate(1));
        assert_eq!(db.get_all_perm_service(1).unwrap(), map);
    }

    /// Assert that our serialization works as intended
    ///
    /// This also revealed that we need to enforce big endian for this to work
    #[test]
    #[ignore]
    fn test_range_service_perm() {
        let config = Config::default().temporary(true);

        let bincode = bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .with_no_limit()
            .with_big_endian();

        let db = config.open().unwrap();
        // take values that require > 1 byte
        db.insert(
            bincode.serialize(&(1, 2)).unwrap(),
            bincode.serialize(&format!("{}-{}", 1, 2)).unwrap(),
        )
        .unwrap();
        db.insert(
            bincode.serialize(&(2, 1)).unwrap(),
            bincode.serialize(&format!("{}-{}", 2, 1)).unwrap(),
        )
        .unwrap();
        db.insert(
            bincode.serialize(&(512, 420)).unwrap(),
            bincode.serialize(&format!("{}-{}", 512, 420)).unwrap(),
        )
        .unwrap();
        db.insert(
            bincode.serialize(&(420, 512)).unwrap(),
            bincode.serialize(&format!("{}-{}", 420, 512)).unwrap(),
        )
        .unwrap();

        let iter =
            db.range(bincode.serialize(&(1, 1)).unwrap()..bincode.serialize(&(513, 513)).unwrap());
        let mut i = 0;
        for elem in iter {
            let (key, val) = elem.unwrap();
            let (a, b) = bincode.deserialize::<Pair>(&key).unwrap();
            println!("{} {}", a, b);
            assert_eq!(
                format!("{}-{}", a, b),
                bincode.deserialize::<String>(&val).unwrap()
            );
            i += 1;
        }
        assert_eq!(4, i);

        // now test a sub-range
        db.insert(
            bincode.serialize(&(420, 510)).unwrap(),
            bincode.serialize(&format!("{}-{}", 420, 510)).unwrap(),
        )
        .unwrap();
        db.insert(
            bincode.serialize(&(420, 509)).unwrap(),
            bincode.serialize(&format!("{}-{}", 420, 509)).unwrap(),
        )
        .unwrap();
        db.insert(
            bincode.serialize(&(420, 508)).unwrap(),
            bincode.serialize(&format!("{}-{}", 420, 508)).unwrap(),
        )
        .unwrap();

        let iter = db.range(
            bincode.serialize(&(420, 508)).unwrap()..bincode.serialize(&(420, 510)).unwrap(),
        );
        let mut i = 0;
        for elem in iter {
            let (key, val) = elem.unwrap();
            let (a, b) = bincode.deserialize::<Pair>(&key).unwrap();
            println!("{} {}", a, b);
            assert_eq!(
                format!("{}-{}", a, b),
                bincode.deserialize::<String>(&val).unwrap()
            );
            i += 1;
        }
        assert_eq!(2, i);
    }
}
