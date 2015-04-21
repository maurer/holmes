use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;

#[test]
pub fn new_predicate_basic() {
  server_single(&|client: &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    }).unwrap();
  })
}

#[test]
pub fn double_register() {
  server_single(&|client : &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]      
    }).unwrap();
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]            
    }).unwrap();
  })
}

#[test]
pub fn double_register_incompat() {
  server_single(&|client : &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]            
    }).unwrap();
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, HString, UInt64]
    }).unwrap_err();
  })
}

#[test]
pub fn pred_persist() {
  server_wrap(vec![&|client : &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    }).unwrap();
  }, &|client : &mut Client| {
    &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, HString, UInt64]
    }).unwrap_err();
  }]);
}
