use std::sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT};
use std::sync::atomic::Ordering::SeqCst;

pub use holmes::*;
pub use holmes::db_types::values::ToValue;
pub use holmes::db_types::values::Value;
pub use holmes::db_types::types::Type;
pub use std::sync::Arc;
pub use holmes::db_types::values;

static DB_NUM : AtomicIsize = ATOMIC_ISIZE_INIT;

pub fn single<A>(test : &Fn(&mut Holmes) -> Result<A>) {
  let port_num = DB_NUM.fetch_add(1, SeqCst);
  let db_addr = format!("postgresql://holmes:holmes@localhost/holmes_test{}", port_num);
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
