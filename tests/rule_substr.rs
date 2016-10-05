use common::*;

#[test]
pub fn simple_substr() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(uint64, bytes));
      predicate!(sub(uint64, bytes));
      fact!(test_pred(1, vec![3u8, 2u8, 1u8]));
      fact!(test_pred(2, vec![1u8, 2u8, 3u8]));
      rule!(sub(n, x) <= test_pred(n, {(1), (2), x}))
    }));
    assert_eq!(query!(holmes, sub((1), x)).unwrap(),
               vec![vec![(vec![2u8, 1u8]).to_value()]]);
    assert_eq!(query!(holmes, sub((2), x)).unwrap(),
               vec![vec![(vec![2u8, 3u8]).to_value()]]);
    Ok(())
  })
}

#[test]
pub fn param_substr() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(uint64, bytes));
      predicate!(sub(uint64, bytes));
      fact!(test_pred(1, vec![3u8, 2u8, 1u8]));
      fact!(test_pred(2, vec![1u8, 2u8, 3u8]));
      rule!(sub(n, x) <= test_pred(n, {[n], (2), x}))
    }));
    assert_eq!(query!(holmes, sub((1), x)).unwrap(),
               vec![vec![(vec![2u8, 1u8]).to_value()]]);
    assert_eq!(query!(holmes, sub((2), x)).unwrap(),
               vec![vec![(vec![3u8]).to_value()]]);
    Ok(())
  })
}
