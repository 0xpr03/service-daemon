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

pub trait DBInterface {
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
    fn set_perm_service(&self, id: UID, service: SID, newPerms: ServicePerm) -> Result<()>;
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
