use std::sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT};
use std::sync::atomic::Ordering::SeqCst;
use std::env;
use url::percent_encoding::{percent_encode, PATH_SEGMENT_ENCODE_SET};
pub use std::sync::Arc;

pub use holmes::*;
pub use holmes::pg::dyn::values::ToValue;
pub use holmes::pg::dyn::{Value, Type};
pub use holmes::pg::dyn::values;

static DB_NUM : AtomicIsize = ATOMIC_ISIZE_INIT;

fn url_encode(input : &[u8]) -> String {
  percent_encode(input, PATH_SEGMENT_ENCODE_SET).to_string()
}

fn get_db_addr(db_num : isize) -> String {
  match env::var("HOLMES_PG_SOCK_DIR") {
    Ok(dir) => format!("postrgresql://holmes@{}/holmes_test{}", url_encode(&dir.into_bytes()), db_num),
    _ => panic!("Testing requires HOLMES_PG_SOCK_DIR to be set to indicate the directory where it can find the postgres database socket.")
  }
}

pub fn single<A>(test : &Fn(&mut Holmes) -> Result<A>) {
  let db_num = DB_NUM.fetch_add(1, SeqCst);
  let db_addr = get_db_addr(db_num);
  println!("{}", db_addr);
  let db = DB::Postgres(db_addr);
  let mut holmes = Holmes::new(db.clone()).unwrap();
  test(&mut holmes).unwrap();
  holmes.destroy().unwrap();
}

pub fn should_fail<A, F>(f : F) -> Box<Fn(&mut Holmes) -> Result<()>>
  where F : 'static + Fn(&mut Holmes) -> Result<A> {
  Box::new(move|holmes : &mut Holmes| {
    match f(holmes) {
      Ok(_) => Err(Error::NoDB), //TODO put something more reasonable here?
      Err(_) => Ok(())
    }
  })
}
