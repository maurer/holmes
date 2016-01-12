use common::*;

#[test]
pub fn one_step() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(string, blob, uint64));
      fact!(test_pred("foo", vec![3;3], 7));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], x))
    }));
    assert_eq!(query!(holmes, test_pred(("bar"), [_], x)).unwrap(),
               vec![vec![7.to_hvalue()]]);
    Ok(())
  })
}

#[test]
pub fn closure() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(reaches(string, string));
      fact!(reaches("foo", "bar"));
      fact!(reaches("bar", "baz"));
      fact!(reaches("baz", "bang"));
      rule!(reaches(src, dst) <= reaches(src, mid) & reaches(mid, dst))
    }));
    let ans = try!(query!(holmes, reaches(("foo"), tgt)));
    assert_eq!(ans, vec![["bar".to_hvalue()], ["baz".to_hvalue()], ["bang".to_hvalue()]]);
    Ok(())
  })
}
