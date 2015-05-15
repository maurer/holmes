use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HValue::*;

#[test]
pub fn register_where_rule() {
  server_single(&|client : &mut Client| {
    client_exec!(client, {
      predicate!(test_pred(string, blob, uint64));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], x), {
        let (42) = (42)})
    })
  })
}

#[test]
pub fn where_const() {
  server_single(&|client : &mut Client| {
    client_exec!(client, {
      predicate!(test_pred(string, blob, uint64));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], [_]), {
          let x = (42)
      });
      fact!(test_pred("foo", vec![0;1], 16))
    });
    assert_eq!(derive!(client, test_pred(("bar"), x, y)).unwrap(),
               vec![vec![vec![2;2].to_hvalue(), 42.to_hvalue()]])
  })
}

#[test]
pub fn where_plus_two() {
  server_single(&|client : &mut Client| {
    client_exec!(client, {
      predicate!(test_pred(string, blob, uint64));
      func!(let plus_two : [uint64] -> [uint64] = |v : Vec<HValue>| {
        match v[0] {
          UInt64V(n) => vec![UInt64V(n + 2)],
          _ => panic!("BAD TYPE")
        }
      });
      rule!(test_pred(("bar"), (vec![2;2]), y) <= test_pred(("foo"), [_], x), {
        let y = {plus_two([x])}
      });
      fact!(test_pred("foo", vec![0;1], 16))
    });
    assert_eq!(derive!(client, test_pred(("bar"), x, y)).unwrap(),
               vec![vec![vec![2;2].to_hvalue(), 18.to_hvalue()]])
  })
}


