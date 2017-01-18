//! You likely don't want to use this module - its primary purpose is to make
//! benchmarking and testing easier to do in practice.

use std::sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT};
use std::sync::atomic::Ordering::SeqCst;
use std::env;
use url::percent_encoding::{percent_encode, PATH_SEGMENT_ENCODE_SET};
pub use std::sync::Arc;

pub use super::pg::dyn::values::ToValue;
pub use super::pg::dyn::{Value, Type};
pub use super::pg::dyn::values;
pub use super::engine::types::{Fact, Rule, Clause, MatchExpr};

use super::PgDB;

pub use engine::Result;
/// Convenience type alias describing the `Engine` specialized to Postgres
pub type Engine = super::Engine<super::pg::Error, PgDB>;

static DB_NUM: AtomicIsize = ATOMIC_ISIZE_INIT;

fn url_encode(input: &[u8]) -> String {
    percent_encode(input, PATH_SEGMENT_ENCODE_SET).to_string()
}

fn get_db_addr(db_num: isize) -> String {
    match env::var("HOLMES_PG_SOCK_DIR") {
        Ok(dir) => {
            format!("postgresql://holmes@{}/holmes_test{}",
                    url_encode(&dir.into_bytes()),
                    db_num)
        }
        _ => {
            panic!("Testing requires HOLMES_PG_SOCK_DIR to be set to \
                     indicate the directory where it can find the postgres \
                     database socket.")
        }
    }
}

/// Call a sequence of functions on the database, simulating a program
/// termination in between each by constructing a fresh `Engine`.
/// Data is _destroyed_ unless an error occurs.
pub fn multi<A>(tests: &[&Fn(&mut Engine) -> Result<A>]) {
    let db_num = DB_NUM.fetch_add(1, SeqCst);
    let db_addr = get_db_addr(db_num);
    for test in tests {
        let db = PgDB::new(&db_addr).unwrap();
        let mut holmes = Engine::new(db);
        test(&mut holmes).unwrap();
    }
    PgDB::destroy(&db_addr).unwrap();
}

/// Convenience wrapper around `multi` which just runs a single function
/// Data is _destroyed_ unless an error occurs.
pub fn single<A>(test: &Fn(&mut Engine) -> Result<A>) {
    multi(&[test])
}

/// Panics on success, and suppresses an error
pub fn should_fail<A, F>(f: F) -> Box<Fn(&mut Engine) -> Result<()>>
    where F: 'static + Fn(&mut Engine) -> Result<A>
{
    Box::new(move |holmes: &mut Engine| {
        match f(holmes) {
            Ok(_) => panic!("should_fail"), //TODO put something more reasonable here?
            Err(_) => Ok(()),
        }
    })
}