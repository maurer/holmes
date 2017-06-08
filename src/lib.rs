//! Holmes
//!
//! Holmes is a Datalog inspired system for binding codependent analyses
//! together.
//!
#![warn(missing_docs)]
#[macro_use]
extern crate lazy_static;
extern crate sha2;
extern crate postgres;
extern crate fallible_iterator;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rustc_serialize;
#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate tokio_core;
extern crate futures;

extern crate env_logger;
extern crate url;

pub mod pg;
pub mod engine;
pub mod edsl;
pub mod simple;

pub use engine::{Engine, Error, ErrorKind, Result};
pub use pg::PgDB;
