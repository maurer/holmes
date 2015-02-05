use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;
use holmes::native_types::HValue::*;

#[test]
pub fn new_fact_basic() {
  server_single(&|&: client : &mut Client| {
    assert!(&client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    }));
    &client.new_fact(&Fact {
      pred_name : "test_pred".to_string(),
      args : vec![HStringV("foo"),
                  BlobV(&[3;3]),
                  UInt64V(7)
                 ]
    }).unwrap();
  })
}

#[test]
pub fn new_fact_type_err() {
  server_single(&|&: client : &mut Client| {
    assert!(&client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    }));
    assert_eq!(&client.new_fact(&Fact {
      pred_name : "test_pred".to_string(),
      args : vec![UInt64V(7),
                  BlobV(&[3;3]),
                  UInt64V(7)
                 ]
    }).unwrap_err()[], "Type mismatch");
  })
}
