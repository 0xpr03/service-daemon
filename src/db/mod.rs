pub mod models;
#[cfg(feature = "sled")]
mod local;
#[cfg(feature = "sled")]
use local::{DBError,DB};
#[cfg(feature = "mysql")]
mod remote;
#[cfg(feature = "mysql")]
use remote::{DBError,DB};

use crate::web::models::*;

pub type Result<T> = ::std::result::Result<T,DBError>;

trait DBInterface {
    fn create_user(&self, ) -> Result<NewUser>;
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