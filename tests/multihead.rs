use common::*;

use holmes::client::*;
use holmes::native_types::*;

#[test]
pub fn basic_multihead() {
  server_single(&|client : &mut Client| {
    client_exec!(client, {
      predicate!(out_a(string));
      predicate!(out_b(string));
      predicate!(inf(string));
      fact!(inf("foo"));
      rule!(out_a(x), out_b(x) <= inf(x))
    });
    assert_eq!(derive!(client, out_a(x)).unwrap(),
               vec![vec!["foo".to_hvalue()]]);
    assert_eq!(derive!(client, out_b(x)).unwrap(),
               vec![vec!["foo".to_hvalue()]]);
  })
}
