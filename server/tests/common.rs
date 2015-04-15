use holmes::server_control::*;
use holmes::client::*;
use std::sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT};
use std::sync::atomic::Ordering::SeqCst;

static PORT : AtomicIsize = ATOMIC_ISIZE_INIT;

pub fn server_wrap(test : Vec<&Fn(&mut Client) -> ()>) {
  let port_num = PORT.fetch_add(1, SeqCst);
  let addr = format!("127.0.0.1:{}", 13370 + port_num);
  let db_addr = format!("postgresql://postgres@localhost/holmes_test{}", port_num);
  let db = DB::Postgres(db_addr);
  {
    let mut server = 
        Server::new(addr.as_slice(), db);
    &server.boot().unwrap();
    for action in test.iter() {
      let mut client = Client::new(addr.as_slice()).unwrap();
      action(&mut client);
      &server.reboot().unwrap();
    }
    &server.destroy().unwrap();
  }
}

pub fn server_single(test : &Fn(&mut Client) -> ()) {
  server_wrap(vec![test])
}
