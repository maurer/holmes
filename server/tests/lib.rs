extern crate holmes;

use holmes::server_control::*;
use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;

#[test]
pub fn new_predicate_basic() {
  let addr = "127.0.0.1:8080";
  let mut server =
      Server::new(addr,
                  DB::Postgres("postgresql://maurer@localhost/holmes_test"));
  unwrap(&server.boot());
  {
    let mut client = Client::new(addr).unwrap();
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    });
  }
  &server.destroy();
}
