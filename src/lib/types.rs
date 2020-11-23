pub use failure::{format_err, Fail, ResultExt, Error};
pub type Result<T> = std::result::Result<T, Error>;
