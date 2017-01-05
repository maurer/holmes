use std::sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT};
use std::sync::atomic::Ordering::SeqCst;
use std::env;
use url::percent_encoding::{percent_encode, PATH_SEGMENT_ENCODE_SET};
pub use std::sync::Arc;

pub use holmes::pg::dyn::values::ToValue;
pub use holmes::pg::dyn::{Value, Type};
pub use holmes::pg::dyn::values;

use holmes::PgDB;

pub type Result<T> =
    ::std::result::Result<T, ::holmes::engine::Error<::holmes::pg::error::Error>>;
pub type Engine = ::holmes::Engine<::holmes::pg::error::Error, PgDB>;

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

pub fn single<A>(test: &Fn(&mut Engine) -> Result<A>) {
    multi(&[test])
}

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
