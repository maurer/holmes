use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;
use holmes::native_types::HValue::*;
use holmes::native_types::MatchExpr::*;

#[test]
pub fn one_step() {
  server_single(&|&: client : &mut Client| {
    let test_pred = "test_pred".to_string();
    assert!(&client.new_predicate(&Predicate {
      name  : test_pred.clone(),
      types : vec![HString, Blob, UInt64]
    }));
    &client.new_fact(&Fact {
      pred_name : test_pred.clone(),
      args : vec![HStringV("foo".to_string()),
                  BlobV(vec![3;3]),
                  UInt64V(7)
                 ]
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
        }]
      };
    &client.new_rule(&rule).unwrap();
    assert_eq!(&client.derive(vec![&Clause {
      pred_name : test_pred,
      args : vec![HConst(HStringV("bar".to_string())),
                  Unbound,
                  Var(0)]
    }]).unwrap(), &vec![vec![UInt64V(7)]]);
  })
}


