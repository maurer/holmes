use std::sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT};
use std::sync::atomic::Ordering::SeqCst;

pub use holmes::*;
pub use holmes::native_types::{ToHValue, HValue, Expr};

static DB_NUM : AtomicIsize = ATOMIC_ISIZE_INIT;

pub fn wrap<A>(test : Vec<&Fn(&mut Holmes) -> Result<A>>) {
  let port_num = DB_NUM.fetch_add(1, SeqCst);
  let db_addr = format!("postgresql://holmes:holmes@localhost/holmes_test{}", port_num);
  let db = DB::Postgres(db_addr);
  for action in test.iter() {
    let mut holmes = Holmes::new(db.clone()).unwrap();
    action(&mut holmes).unwrap();
  }
  Holmes::new(db).unwrap().destroy().unwrap();
}

pub fn single<A>(test : &Fn(&mut Holmes) -> Result<A>) {
  wrap(vec![test])
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
