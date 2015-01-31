extern crate holmes;

use holmes::server_control::*;
use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;

fn server_wrap(test : Vec<&Fn(&mut Client) -> ()>) {
  let addr = "127.0.0.1:8080";
  let mut server = 
      Server::new(addr,
                  DB::Postgres("postgresql://maurer@localhost/holmes_test"));
  unwrap(&server.boot());
  for action in test.iter() {
    let mut client = Client::new(addr).unwrap();
    action(&mut client);
    unwrap(&server.reboot());
  }
  &server.destroy();
}

fn server_single(test : &Fn(&mut Client) -> ()) {
  server_wrap(vec![test])
}

#[test]
pub fn new_predicate_basic() {
  server_single(&|&: client : &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    });
  })
}
