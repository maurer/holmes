use std::convert::From;
use std::fmt::{Formatter};
use std::fmt;
use postgres as pg;

#[derive(Debug)]
pub enum Error {
  Connect(pg::error::ConnectError),
  Db(pg::error::Error),
  Type(String),
  Internal(String),
  Arg(String)
}

impl From<pg::error::Error> for Error {
  fn from(x : pg::error::Error) -> Error {Error::Db(x)}
}
impl From<pg::error::ConnectError> for Error {
  fn from(x : pg::error::ConnectError) -> Error {Error::Connect(x)}
}

impl ::std::fmt::Display for Error {
  fn fmt (&self, fmt : &mut Formatter) -> fmt::Result {
    match *self {
      Error::Connect(ref x) => x.fmt(fmt),
      Error::Db(ref x) => x.fmt(fmt),
      Error::Type(ref s) =>
        fmt.write_str(&format!("Could not parse db type: {}",
                               s.clone())),
      Error::Internal(ref s) =>
        fmt.write_str(&format!("PgDB Internal Error: {}",
                               s.clone())),
      Error::Arg(ref s) =>
        fmt.write_str(&format!("Bad argument: {}", s))
    }
  }
}

impl ::std::error::Error for Error {
  fn description(&self) -> &str {
    match *self {
      Error::Connect(ref x) => x.description(),
      Error::Db(ref x) => x.description(),
      Error::Type(_) => "Could not parse db types",
      Error::Internal(_) => "PgDB Internal Error",
      Error::Arg(_) => "Bad argument: {}"
    }
  }
}

pub type Result<T> = ::std::result::Result<T, Error>;
