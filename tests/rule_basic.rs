use common::*;

use holmes::client::*;
use holmes::native_types::*;

#[test]
pub fn one_step() {
  server_single(&|client : &mut Client| {
    client_exec!(client, {
      predicate!(test_pred(string, blob, uint64));
      fact!(test_pred("foo", vec![3;3], 7));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], x))
    });
    assert_eq!(derive!(client, test_pred(("bar"), [_], x)).unwrap(),
               vec![vec![7.to_hvalue()]])
  })
}

#[test]
pub fn closure() {
  server_single(&|client : &mut Client| {
    client_exec!(client, {
      predicate!(reaches(string, string));
      fact!(reaches("foo", "bar"));
      fact!(reaches("bar", "baz"));
      fact!(reaches("baz", "bang"));
      rule!(reaches(src, dst) <= reaches(src, mid) & reaches(mid, dst))
    });
    let ans = derive!(client, reaches(("foo"), tgt)).unwrap();
    assert_eq!(ans, vec![["bar".to_hvalue()], ["baz".to_hvalue()], ["bang".to_hvalue()]]);
  })
}

