#[cfg(feature = "local")]
mod local;
pub mod models;
use local::{DBError, DB as InnerDB};
#[cfg(feature = "remote")]
mod remote;
#[cfg(feature = "remote")]
use remote::{DBError, DB as InnerDB};

use crate::web::models::*;
use models::*;
use std::collections::HashMap;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "Internal DB error {}", _0)]
    InternalError(DBError),
    #[fail(display = "The specified user id {} is invalid!", _0)]
    InvalidUser(UID),
    #[fail(display = "User mail exists already!")]
    EMailExists,
}

impl From<DBError> for Error {
    fn from(error: DBError) -> Self {
        Error::InternalError(error)
    }
}

fn get_current_time() -> u64 {
    ::std::time::SystemTime::now()
        .duration_since(::std::time::UNIX_EPOCH)
        .expect("Invalid SystemTime!")
        .as_secs()
}

pub trait DBInterface: Sized {
    fn new_temp() -> Self;
    /// Create new User
    fn create_user(&self, user: NewUserEnc) -> Result<FullUser>;
    /// Delete user & active logins
    fn delete_user(&self, id: UID) -> Result<()>;
    /// Get user by UID
    fn get_user(&self, id: UID) -> Result<FullUser>;
    /// Get UID by email
    fn get_id_by_email(&self, email: &str) -> Result<Option<UID>>;
    /// Update user settings
    fn update_user(&self, user: FullUser) -> Result<()>;
    /// Get all users in min representation
    fn get_users(&self) -> Result<Vec<UserMin>>;
    /// Get all admin UIDs
    fn get_perm_admin(&self) -> Result<Vec<UID>>;
    /// Get user permissions for a service
    fn get_perm_service(&self, id: UID, service: SID) -> Result<ServicePerm>;
    /// Get all service permissions of a user
    fn get_all_perm_service(&self, id: UID) -> Result<HashMap<SID, ServicePerm>>;
    /// Update user permissions for service
    fn set_perm_service(&self, id: UID, service: SID, new_perms: ServicePerm) -> Result<()>;
    /// Get session login if not older than max_age
    fn get_login(&self, session: &str, max_age: u32) -> Result<Option<ActiveLogin>>;
    /// Set session login
    fn set_login(&self, session: &str, state: Option<ActiveLogin>) -> Result<()>;
    /// Update session login timestamp  
    ///
    /// session may be invalid (get_login -> None), as it makes no difference.
    fn update_login(&self, session: &str) -> Result<()>;
    /// Delete logins older than max_age
    fn delete_old_logins(&self, max_age: u32) -> Result<usize>;
    /// Get (reserved) root UID
    fn get_root_id(&self) -> UID;
    /// Insert new service log entry, returns log entry ID
    fn insert_log_entry(
        &self,
        service: SID,
        entry: NewLogEntry,
        console: Option<ConsoleOutput>,
    ) -> Result<()>;
    /// Get last n log entries for service
    fn service_log_limited(&self, service: SID, limit: usize) -> Result<Vec<LogEntryResolved>>;
    /// Get service log entries between two dates
    fn service_log_date(&self, service: SID, from: Date, to: Date) -> Result<Vec<LogEntry>>;
    /// Returns min and max dates where log entries exist for a given service
    ///
    /// Returns none if no entries exist
    fn service_log_minmax(&self, service: SID) -> Result<Option<(Date, Date)>>;
    /// Get service log console content
    fn get_service_console_log(&self, service: SID, log_id: LogID)
        -> Result<Option<ConsoleOutput>>;
    /// Return service log entry details
    fn get_service_log_details(
        &self,
        service: SID,
        log_id: LogID,
    ) -> Result<Option<LogEntryResolved>>;
    /// List all services for which
    fn cleanup(&self, max_age: Date) -> Result<()>;
}

lazy_static! {
    pub static ref DB: InnerDB = InnerDB::default();
}

macro_rules! assert_unique_feature {
    () => {};
    ($first:tt $(,$rest:tt)*) => {
        $(
            #[cfg(all(feature = $first, feature = $rest))]
            compile_error!(concat!("features \"", $first, "\" and \"", $rest, "\" cannot be used together"));
        )*
        assert_unique_feature!($($rest),*);
    }
}

assert_unique_feature!("local", "remote");
#[cfg(not(any(feature = "local", feature = "remote")))]
compile_error!("Either feature \"local\" or \"remote\" must be enabled for this crate.");

#[cfg(test)]
mod test {
    use super::*;
    use rand::{thread_rng, RngCore};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread::sleep;
    use std::time::Duration;

    static USER_UNIQUE: AtomicUsize = AtomicUsize::new(0);

    fn gen_user() -> NewUserEnc {
        let r = thread_rng().next_u32();
        let id = USER_UNIQUE.fetch_add(1, Ordering::SeqCst);
        NewUserEnc {
            name: format!("{}Name{}", id, r),
            password_enc: format!("{}Password{}", id, r),
            email: format!("{}Email{}", id, r),
        }
    }
    fn gen_db() -> InnerDB {
        let db = InnerDB::new_temp();
        assert_eq!(0, db.get_users().unwrap().len());
        db
    }
    /// creates a user and returns full representation
    fn create_user(db: &InnerDB) -> (NewUserEnc, FullUser) {
        let user_new = gen_user();
        let full_user = db.create_user(user_new.clone()).unwrap();
        (user_new, full_user)
    }

    #[test]
    fn test_temp_db() {
        let _ = InnerDB::new_temp();
    }
    #[test]
    fn test_create_user() {
        let db = gen_db();
        let (user_new, full_user) = create_user(&db);
        assert_eq!(full_user.name, user_new.name);
        assert_eq!(full_user.password, user_new.password_enc);
        assert_eq!(full_user.email, user_new.email);
        assert_eq!(full_user.totp_setup_complete, false);
        assert_eq!(false, full_user.admin);
        assert!(!full_user.totp.secret.is_empty());

        let uid_mail = db.get_id_by_email(&user_new.email).unwrap();
        assert_eq!(Some(full_user.id), uid_mail);

        assert_eq!(1, db.get_users().unwrap().len());
    }

    #[test]
    fn test_update_user_mail() {
        let db = gen_db();
        let (user_new, full_user) = create_user(&db);

        let mut user_updated = full_user.clone();
        user_updated.email = String::from("test");
        assert_ne!(user_updated.email, full_user.email);

        // change email
        db.update_user(user_updated.clone()).unwrap();
        assert_eq!(None, db.get_id_by_email(&user_new.email).unwrap());
        assert_eq!(
            Some(full_user.id),
            db.get_id_by_email(&user_updated.email).unwrap()
        );

        // email stays the same
        user_updated.name = String::from("testName");
        db.update_user(user_updated.clone()).unwrap();
        assert_eq!(None, db.get_id_by_email(&user_new.email).unwrap());
        assert_eq!(
            Some(full_user.id),
            db.get_id_by_email(&user_updated.email).unwrap()
        );
    }

    #[test]
    fn test_update_email_duplicate() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);
        let (_, mut other_user) = create_user(&db);
        other_user.email = full_user.email;
        match db.update_user(other_user) {
            Err(Error::EMailExists) => (),
            v => panic!("Expected EmailExists, got {:?}", v),
        }
    }

    #[test]
    fn test_creation_email_duplicate() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);
        let new_user = NewUserEnc {
            name: "something".to_string(),
            password_enc: "asdf".to_string(),
            email: full_user.email,
        };
        match db.create_user(new_user) {
            Err(Error::EMailExists) => (),
            v => panic!("Expected EmailExists, got {:?}", v),
        }
    }

    #[test]
    fn test_update_user() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);

        let mut user_ch = full_user.clone();
        user_ch.name = String::from("change-Name");
        // user_ch.id = String::from("change-Name");
        user_ch.password = String::from("change-password");
        user_ch.email = String::from("change-email");
        user_ch.verified = false;
        user_ch.totp.secret = String::from("change-secret").into_bytes();
        user_ch.totp_setup_complete = true;
        assert_ne!(user_ch, full_user);

        db.update_user(user_ch.clone()).unwrap();

        assert_eq!(user_ch, db.get_user(full_user.id).unwrap());
    }

    #[test]
    fn test_invalid_id_update() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);
        let mut invalid_user = full_user.clone();
        let invalid_id = full_user.id + 1;
        invalid_user.id = invalid_id;
        match db.get_user(invalid_user.id) {
            Err(Error::InvalidUser(_)) => {
                // don't allow user creation sidechannel
                match db.update_user(invalid_user.clone()) {
                    Err(Error::InvalidUser(id)) => assert_eq!(id, invalid_id),
                    v => panic!("Expected invalid user! {:?}", v),
                }
            }
            v => panic!("Expected invalid ID! Got {:?}", v),
        }
    }

    #[test]
    fn test_set_login_session() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);
        let sess = String::from("session");
        let state = ActiveLogin {
            id: full_user.id,
            state: LoginState::Complete,
        };
        db.set_login(&sess, Some(state.clone())).unwrap();
        assert_eq!(Some(state), db.get_login(&sess, 12).unwrap());
        db.set_login(&sess, None).unwrap();
        assert_eq!(None, db.get_login(&sess, 12).unwrap());
    }

    #[test]
    fn test_login_update() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);
        let sess = String::from("session");
        let state = ActiveLogin {
            id: full_user.id,
            state: LoginState::Complete,
        };
        db.set_login(&sess, Some(state.clone())).unwrap();
        sleep(Duration::from_millis(1_000));
        db.update_login(&sess).unwrap();
        assert_eq!(Some(state), db.get_login(&sess, 0).unwrap());
    }

    #[test]
    fn test_session_login_timeout() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);
        let sess = String::from("session");
        let state = ActiveLogin {
            id: full_user.id,
            state: LoginState::Complete,
        };
        db.set_login(&sess, Some(state.clone())).unwrap();
        assert_eq!(Some(state), db.get_login(&sess, 12).unwrap());
        sleep(Duration::from_millis(1_000));
        assert_eq!(None, db.get_login(&sess, 0).unwrap());
        assert_eq!(1, db.delete_old_logins(0).unwrap());
    }

    #[test]
    fn test_perm_service_unknown() {
        let db = gen_db();
        assert!(db.get_perm_service(1, 1).unwrap().is_empty());
    }

    #[test]
    fn test_perm_service() {
        let db = gen_db();
        let (_, full_user) = create_user(&db);
        let perm = ServicePerm::from_bits(0b00001010).unwrap();
        db.set_perm_service(full_user.id, 1, perm.clone()).unwrap();
        assert_eq!(perm, db.get_perm_service(full_user.id, 1).unwrap());
    }

    #[test]
    fn test_perm_admin() {
        let db = gen_db();
        let (_, mut full_user) = create_user(&db);
        let id = full_user.id.clone();
        full_user.admin = true;
        db.update_user(full_user).unwrap();
        assert_eq!(true, db.get_user(id).unwrap().admin);
    }
}
