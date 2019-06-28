#[cfg(feature = "sled")]
mod local;
pub mod models;
use local::{DBError, DB as InnerDB};
#[cfg(feature = "mysql")]
mod remote;
#[cfg(feature = "mysql")]
use remote::{DBError, DB as InnerDB};

use crate::web::models::*;
use models::*;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "Internal DB error {}", _0)]
    InternalError(DBError),
    #[fail(display = "The specified user id {} is invalid!", _0)]
    InvalidUser(UID),
    #[fail(display = "User mail {} exists already!", _0)]
    EMailExists(String),
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
    fn create_user(&self, user: NewUserEncrypted) -> Result<FullUser>;
    /// Delete user & active logins
    fn delete_user(&self, id: UID) -> Result<()>;
    /// Get user by UID
    fn get_user(&self, id: UID) -> Result<FullUser>;
    /// Get UID by email
    fn get_id_by_email(&self, email: &str) -> Result<Option<UID>>;
    /// Update user settings
    fn update_user(&self, user: FullUser) -> Result<()>;
    /// Get all users in min representation
    fn get_users(&self) -> Result<Vec<MinUser>>;
    /// Get user permissions for management
    fn get_perm_man(&self, id: UID) -> Result<ManagementPerm>;
    /// Set user permissions for management
    fn set_perm_man(&self, id: UID, perm: &ManagementPerm) -> Result<()>;
    /// Get user permissions for a service
    fn get_perm_service(&self, id: UID, service: SID) -> Result<ServicePerm>;
    /// Update user permissions for service
    fn set_perm_service(&self, id: UID, service: SID, new_perms: ServicePerm) -> Result<()>;
    /// Get session login if not older than max_age
    fn get_login(&self, session: &str, max_age: u32) -> Result<Option<ActiveLogin>>;
    /// Set session login
    fn set_login(&self, session: &str, state: Option<ActiveLogin>) -> Result<()>;
    /// Update session login timestamp
    fn update_login(&self, session: &str) -> Result<()>;
    /// Delete logins older than max_age
    fn delete_old_logins(&self, max_age: u32) -> Result<usize>;
    /// Get (reserved) root UID
    fn get_root_id(&self) -> UID;
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
    fn gen_user() -> NewUserEncrypted {
        use rand::*;
        let r = thread_rng().next_u32();
        NewUserEncrypted {
            name: format!("Name{}", r),
            password_enc: format!("Password{}", r),
            email: format!("Email{}", r),
        }
    }

    #[test]
    fn test_temp_db() {
        let _ = InnerDB::new_temp();
    }
    #[test]
    fn test_create_user() {
        let db = InnerDB::new_temp();
        assert_eq!(0, db.get_users().unwrap().len());
        let user_new = gen_user();
        let full_user = db.create_user(user_new.clone()).unwrap();
        assert_eq!(full_user.name, user_new.name);
        assert_eq!(full_user.password, user_new.password_enc);
        assert_eq!(full_user.email, user_new.email);
        assert_eq!(full_user.totp_complete, false);
        assert!(!full_user.totp.secret.is_empty());

        let uid_mail = db.get_id_by_email(&user_new.email).unwrap();
        assert_eq!(Some(full_user.id), uid_mail);

        assert_eq!(1, db.get_users().unwrap().len());

        let perm_man = db.get_perm_man(full_user.id).unwrap();
        assert_eq!(false, perm_man.admin);
    }

    #[test]
    fn test_update_user_mail() {
        let db = InnerDB::new_temp();
        assert_eq!(0, db.get_users().unwrap().len());
        let user_new = gen_user();
        let full_user = db.create_user(user_new.clone()).unwrap();

        let mut user_updated = full_user.clone();
        user_updated.email = String::from("test");
        assert_ne!(user_updated.email,full_user.email);

        // change email
        db.update_user(user_updated.clone()).unwrap();
        assert_eq!(None,db.get_id_by_email(&user_new.email).unwrap());
        assert_eq!(Some(full_user.id),db.get_id_by_email(&user_updated.email).unwrap());

        // email stays the same
        user_updated.name = String::from("testName");
        db.update_user(user_updated.clone()).unwrap();
        assert_eq!(None,db.get_id_by_email(&user_new.email).unwrap());
        assert_eq!(Some(full_user.id),db.get_id_by_email(&user_updated.email).unwrap());
    }
}
