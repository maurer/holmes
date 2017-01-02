//! Postgres fact database layer error types.

use std::convert::From;
use std::fmt::Formatter;
use std::fmt;
use postgres as pg;

/// The `Error` type for representing things going wrong in the database
/// layer.
#[derive(Debug)]
pub enum Error {
    /// Carrier for Postgres connection errors
    Connect(pg::error::ConnectError),
    /// Carrier for Postgres DB errors.
    /// Usually, this is a bug in the DB layer, but we do so many DB operations
    /// that it's not always clear which error to cast it to. In general, you
    /// can think of this like an `Internal` error, but with more specificity.
    Db(pg::error::Error),
    /// Type conflict errors. Since the database layer is being used to persist
    /// a typed language, these can occur in a number of circumstances, ranging
    /// from attempting to persist a fact using the wrong type, to attempting
    /// to define a new predicate using a type which does not exist.
    Type(String),
    /// An implementation bug in the database layer. Essentially less fatal
    /// assertions. This means the database layer is at fault.
    Internal(String),
    /// Bad input to the database layer. This means the caller is at fault.
    Arg(String),
}

impl From<pg::error::Error> for Error {
    fn from(x: pg::error::Error) -> Error {
        Error::Db(x)
    }
}
impl From<pg::error::ConnectError> for Error {
    fn from(x: pg::error::ConnectError) -> Error {
        Error::Connect(x)
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            Error::Connect(ref x) => x.fmt(fmt),
            Error::Db(ref x) => x.fmt(fmt),
            Error::Type(ref s) => fmt.write_str(&format!("Type Error: {}", s.clone())),
            Error::Internal(ref s) => {
                fmt.write_str(&format!("PgDB Internal Error (Bug): {}", s.clone()))
            }
            Error::Arg(ref s) => fmt.write_str(&format!("Bad argument: {}", s)),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Connect(ref x) => x.description(),
            Error::Db(ref x) => x.description(),
            Error::Type(_) => "Type Error",
            Error::Internal(_) => "PgDB Internal Error",
            Error::Arg(_) => "Bad argument",
        }
    }
    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::Connect(ref x) => Some(x),
            Error::Db(ref x) => Some(x),
            Error::Type(_) |
            Error::Internal(_) |
            Error::Arg(_) => None,
        }
    }
}

/// Type alias for a `Result` using the `Error` type defined above.
pub type Result<T> = ::std::result::Result<T, Error>;
