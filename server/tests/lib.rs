#![feature(core)]
extern crate holmes;

use holmes::server_control::*;
use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;
use std::sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT};
use std::sync::atomic::Ordering::SeqCst;

static PORT : AtomicIsize = ATOMIC_ISIZE_INIT;

fn server_wrap(test : Vec<&Fn(&mut Client) -> ()>) {
  let port_num = PORT.fetch_add(1, SeqCst);
  let addr = format!("127.0.0.1:{}", 13370 + port_num);
  let db_addr = format!("postgresql://postgres@localhost/holmes_test{}", port_num);
  let db = DB::Postgres(db_addr);
  {
    let mut server = 
        Server::new(addr.as_slice(), db);
    unwrap(&server.boot());
    for action in test.iter() {
      let mut client = Client::new(addr.as_slice()).unwrap();
      action(&mut client);
      unwrap(&server.reboot());
    }
    &server.destroy();
  }
}

fn server_single(test : &Fn(&mut Client) -> ()) {
  server_wrap(vec![test])
}

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
