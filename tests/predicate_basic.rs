use common::*;

use holmes::client::*;
use holmes::native_types::*;

#[test]
pub fn new_predicate_basic() {
  server_single(&|client: &mut Client| { client_exec!(client, { 
    predicate!(test_pred(string, blob, uint64))
  })})
}

#[test]
pub fn double_register() {
  server_single(&|client: &mut Client| { client_exec!(client, { 
    predicate!(test_pred(string, blob, uint64));
    predicate!(test_pred(string, blob, uint64))
  })})
}

#[test]
pub fn double_register_incompat() {
  server_single(&|client: &mut Client| { client_exec!(client, { 
    predicate!(test_pred(string, blob, uint64));
    should_fail(predicate!(test_pred(string, string, string)))
  })})
}

#[test]
pub fn pred_persist() {
  server_wrap(vec![&|client : &mut Client| {
    predicate!(client, test_pred(string, blob, uint64)).unwrap();
  }, &|client : &mut Client| {
    predicate!(client, test_pred(string, string, string)).unwrap_err();
  }]);
}
