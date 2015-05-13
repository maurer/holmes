use common::*;

use holmes::client::*;
use holmes::native_types::*;

#[test]
pub fn new_fact_basic() {
  server_single(&|client : &mut Client| { client_exec!(client, {
    predicate!(test_pred(string, blob, uint64));
    fact!(test_pred("foo", vec![3,4,5], 7))
  })})
}

#[test]
pub fn new_fact_type_err() {
  server_single(&|client : &mut Client| { client_exec!(client, {
    predicate!(test_pred(string, blob, uint64));
    should_fail(fact!(test_pred(7, vec![3,4,5], 7)))
  })})
}

#[test]
pub fn new_fact_echo() {
  server_single(&|client : &mut Client| {
    client_exec!(client, {
      predicate!(test_pred(string, blob, uint64));
      fact!(test_pred("foo", vec![3;3], 7))
    });
    assert_eq!(derive!(client,
                       test_pred(("foo"), [_], x)).unwrap(),
               vec![vec![7.to_hvalue()]]);
  })
}

#[test]
pub fn two_strings() {
  server_single(&|client : &mut Client| { client_exec!(client, {
    predicate!(test_pred(string, string));
    fact!(test_pred("foo", "bar"))
  })})
}
