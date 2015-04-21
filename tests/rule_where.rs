use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;
use holmes::native_types::HValue::*;
use holmes::native_types::MatchExpr::*;
use holmes::native_types::Expr::*;

#[test]
pub fn register_where_rule() {
  server_single(&|client : &mut Client| {
    let test_pred = "test_pred".to_string();
    &client.new_predicate(&Predicate {
      name  : test_pred.clone(),
      types : vec![HString, Blob, UInt64]
    }).unwrap();
    let rule = Rule {
      head : Clause {
        pred_name : test_pred.clone(),
        args : vec![HConst(HStringV("bar".to_string())),
                    HConst(BlobV(vec![2;2])),
                    Var(0)]
      },
      body : vec![Clause {
        pred_name : test_pred.clone(),
        args : vec![HConst(HStringV("foo".to_string())),
                    Unbound,
                    Var(0)]
        }],
      wheres: vec![WhereClause {
        asgns : vec![HConst(UInt64V(42))],
        rhs : EVal(UInt64V(42))
      }]
      };
    &client.new_rule(&rule).unwrap();
  })
}

#[test]
pub fn where_const() {
  server_single(&|client : &mut Client| {
    let test_pred = "test_pred".to_string();
    &client.new_predicate(&Predicate {
      name  : test_pred.clone(),
      types : vec![HString, Blob, UInt64]
    }).unwrap();
    let rule = Rule {
      head : Clause {
        pred_name : test_pred.clone(),
        args : vec![HConst(HStringV("bar".to_string())),
                    HConst(BlobV(vec![2;2])),
                    Var(0)]
      },
      body : vec![Clause {
        pred_name : test_pred.clone(),
        args : vec![HConst(HStringV("foo".to_string())),
                    Unbound,
                    Unbound]
        }],
      wheres: vec![WhereClause {
        asgns : vec![Var(0)],
        rhs : EVal(UInt64V(42))
      }]
      };
    &client.new_rule(&rule).unwrap();
    &client.new_fact(&Fact {
      pred_name : test_pred.clone(),
      args : vec![HStringV("foo".to_string()),
                  BlobV(vec![0;1]),
                  UInt64V(16)
                 ]
    }).unwrap();
    assert_eq!(&client.derive(vec![&Clause {
      pred_name : test_pred.clone(),
      args : vec![HConst(HStringV("bar".to_string())),
                  Var(0),
                  Var(1)
                 ]
    }]).unwrap(), &vec![vec![BlobV(vec![2;2]),
                             UInt64V(42)]])
  })
}

#[test]
pub fn where_plus_two() {
  server_single(&|client : &mut Client| {
    let test_pred = "test_pred".to_string();
    &client.new_predicate(&Predicate {
      name  : test_pred.clone(),
      types : vec![HString, Blob, UInt64]
    }).unwrap();
    &client.new_func("test_func", HFunc {
      input_types : vec![UInt64],
      output_types : vec![UInt64],
      run : Box::new(|v : Vec<HValue>| {
        match v[0] {
          UInt64V(n) => vec![UInt64V(n + 2)],
          _ => panic!("BAD TYPE")
        }
      })
    }).unwrap();
    let rule = Rule {
      head : Clause {
        pred_name : test_pred.clone(),
        args : vec![HConst(HStringV("bar".to_string())),
                    HConst(BlobV(vec![2;2])),
                    Var(1)]
      },
      body : vec![Clause {
        pred_name : test_pred.clone(),
        args : vec![HConst(HStringV("foo".to_string())),
                    Unbound,
                    Var(0)]
        }],
      wheres: vec![WhereClause {
        asgns : vec![Var(1)],
        rhs : EApp("test_func".to_string(), vec![EVar(0)])
      }]
      };
    &client.new_rule(&rule).unwrap();
    &client.new_fact(&Fact {
      pred_name : test_pred.clone(),
      args : vec![HStringV("foo".to_string()),
                  BlobV(vec![0;1]),
                  UInt64V(16)
                 ]
    }).unwrap();
    assert_eq!(&client.derive(vec![&Clause {
      pred_name : test_pred.clone(),
      args : vec![HConst(HStringV("bar".to_string())),
                  Var(0),
                  Var(1)
                 ]
    }]).unwrap(), &vec![vec![BlobV(vec![2;2]),
                             UInt64V(18)]])
  })
}


