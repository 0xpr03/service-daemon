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
    #[fail(display = "The specified name {} is invalid!", _0)]
    InvalidName(String),
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
    fn create_user(&self, user: NewUserEncrypted) -> Result<FullUser>;
    fn delete_user(&self, id: UID) -> Result<()>;
    fn get_user(&self, id: UID) -> Result<FullUser>;
    fn get_id_by_email(&self, email: &str) -> Result<Option<UID>>;
    fn update_user(&self, user: FullUser) -> Result<()>;
    fn get_users(&self) -> Result<Vec<MinUser>>;
    fn get_user_permissions(&self, id: UID) -> Result<Vec<String>>;
    fn update_user_permission(&self, id: UID, perms: Vec<String>) -> Result<()>;
    /// Get session login
    fn get_login(&self, session: &str) -> Result<Option<ActiveLogin>>;
    /// Set session login
    fn set_login(&self, session: &str, state: Option<ActiveLogin>) -> Result<()>;
    /// Update session login timestamp
    fn update_login(&self, session: &str) -> Result<()>;
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

assert_unique_feature!("mysql", "sled");
