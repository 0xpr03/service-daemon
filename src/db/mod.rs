#[cfg(feature = "sled")]
mod local;
pub mod models;
use local::{DBError, DB as InnerDB};
#[cfg(feature = "mysql")]
mod remote;
#[cfg(feature = "mysql")]
use remote::{DBError, DB as InnerDB};

use crate::web::models::*;
use actix::prelude::*;
use bcrypt::{hash, verify, BcryptResult, DEFAULT_COST};
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
    #[fail(display = "User name {} exists already!", _0)]
    NameExists(String),
}

fn bcrypt_password(password: &str) -> BcryptResult<String> {
    hash(password, DEFAULT_COST)
}

pub fn bcrypt_verify(password: &str, hash: &str) -> BcryptResult<bool> {
    verify(password, hash)
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
    fn create_user(&self, user: NewUser) -> Result<FullUser>;
    fn delete_user(&self, id: UID) -> Result<()>;
    fn get_user(&self, id: UID) -> Result<FullUser>;
    fn get_id_by_name(&self, name: &str) -> Result<Option<UID>>;
    fn update_user(&self, user: FullUser) -> Result<()>;
    fn get_users(&self) -> Result<Vec<MinUser>>;
    fn get_user_permissions(&self, id: UID) -> Result<Vec<String>>;
    fn update_user_permission(&self, id: UID, perms: Vec<String>) -> Result<()>;
    fn get_login(&self, login: &str) -> Result<Option<ActiveLogin>>;
    fn set_login(&self, login: &str, state: Option<ActiveLogin>) -> Result<()>;
    fn update_login(&self, login: &str) -> Result<()>;
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
