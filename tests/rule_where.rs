use common::*;

#[test]
pub fn register_where_rule() {
  single(&|holmes : &mut Holmes| {
    holmes_exec!(holmes, {
      predicate!(test_pred(string, blob, uint64));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], x), {
        let (42) = (42)})
    })
  })
}

#[test]
pub fn where_const() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(string, blob, uint64));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], [_]), {
          let x = (42)
      });
      fact!(test_pred("foo", vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred(("bar"), x, y)).unwrap(),
               vec![vec![vec![2;2].to_hvalue(), 42.to_hvalue()]]);
    Ok(())
  })
}

#[test]
pub fn where_plus_two() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(string, blob, uint64));
      func!(let plus_two : [uint64] -> uint64 = |v : HValue| {
        match v {
          HValue::UInt64V(n) => HValue::UInt64V(n + 2),
          _ => panic!("BAD TYPE")
        }
      });
      rule!(test_pred(("bar"), (vec![2;2]), y) <= test_pred(("foo"), [_], x), {
        let y = {plus_two([x])}
      });
      fact!(test_pred("foo", vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred(("bar"), x, y)).unwrap(),
               vec![vec![vec![2;2].to_hvalue(), 18.to_hvalue()]]);
    Ok(())
  })
}

#[test]
pub fn where_destructure() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(uint64, blob, uint64));
      func!(let succs : [uint64] -> (uint64, uint64) = |v : HValue| {
        match v {
          HValue::UInt64V(n) => HValue::ListV(vec![
            HValue::UInt64V(n + 1),
            HValue::UInt64V(n + 2)]),
          _ => panic!("BAD TYPE")
        }
      });
      rule!(test_pred(y, (vec![2;2]), z) <= test_pred((3), [_], x), {
        let y, z = {succs([x])}
      });
      fact!(test_pred(3, vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred(y, (vec![2;2]), z)).unwrap(),
               vec![vec![17.to_hvalue(), 18.to_hvalue()]]);
    Ok(())
  })
}

#[test]
pub fn where_iter() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(uint64, blob, uint64));
      func!(let succs : [uint64] -> [uint64] = |v : HValue| {
        match v {
          HValue::UInt64V(n) => HValue::ListV(vec![
            HValue::UInt64V(n + 1),
            HValue::UInt64V(n + 2)]),
          _ => panic!("BAD TYPE")
        }
      });
      rule!(test_pred((4), (vec![2;2]), y) <= test_pred((3), [_], x), {
        let [y] = {succs([x])}
      });
      fact!(test_pred(3, vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred([_], (vec![2;2]), x)).unwrap(),
               vec![vec![17.to_hvalue()], vec![18.to_hvalue()]]);
    Ok(())
  })
}
