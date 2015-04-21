use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;
use holmes::native_types::HValue::*;
use holmes::native_types::MatchExpr::*;

#[test]
pub fn new_fact_basic() {
  server_single(&|client : &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    });
    &client.new_fact(&Fact {
      pred_name : "test_pred".to_string(),
      args : vec![HStringV("foo".to_string()),
                  BlobV(vec![3;3]),
                  UInt64V(7)
                 ]
    }).unwrap();
  })
}

#[test]
pub fn new_fact_type_err() {
  server_single(&|client : &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    }).unwrap();
    &client.new_fact(&Fact {
      pred_name : "test_pred".to_string(),
      args : vec![UInt64V(7),
                  BlobV(vec![3;3]),
                  UInt64V(7)
                 ]
    }).unwrap_err();
  })
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
  server_single(&|client : &mut Client| {
    let test_pred = "test_pred".to_string();
    &client.new_predicate(&Predicate {
      name  : test_pred.clone(),
      types : vec![HString, HString]
    }).unwrap();
    &client.new_fact(&Fact {
      pred_name : test_pred.clone(),
      args : vec![HStringV("foo".to_string()),
                  HStringV("bar".to_string())
                 ]
    }).unwrap();
  })
}
