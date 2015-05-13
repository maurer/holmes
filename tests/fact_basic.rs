use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;
use holmes::native_types::HValue::*;
use holmes::native_types::MatchExpr::*;

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
    let test_pred = "test_pred".to_string();
    &client.new_predicate(&Predicate {
      name  : test_pred.clone(),
      types : vec![HString, Blob, UInt64]
    }).unwrap();
    &client.new_fact(&Fact {
      pred_name : test_pred.clone(),
      args : vec![HStringV("foo".to_string()),
                  BlobV(vec![3;3]),
                  UInt64V(7)
                 ]
    }).unwrap();
    assert_eq!(&client.derive(vec![&Clause {
      pred_name : test_pred,
      args : vec![HConst(HStringV("foo".to_string())),
                  Unbound,
                  Var(0)]
    }]).unwrap(), &vec![vec![UInt64V(7)]]);
  })
}

#[test]
pub fn two_strings() {
  server_single(&|client : &mut Client| { client_exec!(client, {
    predicate!(test_pred(string, string));
    fact!(test_pred("foo", "bar"))
  })})
}
