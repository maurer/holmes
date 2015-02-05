use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;

#[test]
pub fn new_predicate_basic() {
  server_single(&|&: client : &mut Client| {
    assert!(&client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    }));
  })
}

#[test]
pub fn double_register() {
  server_single(&|&: client : &mut Client| {
    let pred1 = &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]      
    });
    let pred2 = &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]            
    });
    assert_eq!(pred1, &true);
    assert_eq!(pred2, &true);
  })
}

#[test]
pub fn double_register_incompat() {
  server_single(&|&: client : &mut Client| {
    let pred1 = &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]            
    });
    let pred2 = &client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, HString, UInt64]
    });
    assert_eq!(pred1, &true);
    assert_eq!(pred2, &false);
  })
}

#[test]
pub fn pred_persist() {
  server_wrap(vec![&|&: client : &mut Client| {
    assert!(&client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, Blob, UInt64]
    }));
  }, &|&: client : &mut Client| {
    assert!(!&client.new_predicate(&Predicate {
      name  : "test_pred".to_string(),
      types : vec![HString, HString, UInt64]
    }));
  }]);
}
