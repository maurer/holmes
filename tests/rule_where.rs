use common::*;

#[test]
pub fn register_where_rule() {
  single(&|holmes : &mut Holmes| {
    holmes_exec!(holmes, {
      predicate!(test_pred(string, bytes, uint64));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], x), {
        let (42) = (42)})
    })
  })
}

#[test]
pub fn where_const() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(string, bytes, uint64));
      rule!(test_pred(("bar"), (vec![2;2]), x) <= test_pred(("foo"), [_], [_]), {
          let x = (42)
      });
      fact!(test_pred("foo", vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred(("bar"), x, y)).unwrap(),
               vec![vec![vec![2;2].to_value(), 42.to_value()]]);
    Ok(())
  })
}

#[test]
pub fn where_plus_two() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(string, bytes, uint64));
      func!(let plus_two : [uint64] -> uint64 = |v : Arc<Value>| {
        match v.get().downcast_ref::<u64>() {
          Some(n) => (n + 2).to_value(),
          _ => panic!("BAD TYPE")
        }
      });
      rule!(test_pred(("bar"), (vec![2;2]), y) <= test_pred(("foo"), [_], x), {
        let y = {plus_two([x])}
      });
      fact!(test_pred("foo", vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred(("bar"), x, y)).unwrap(),
               vec![vec![vec![2;2].to_value(), 18.to_value()]]);
    Ok(())
  })
}

#[test]
pub fn where_destructure() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(uint64, bytes, uint64));
      func!(let succs : [uint64] -> (uint64, uint64) = |v : Arc<Value>| {
        match v.get().downcast_ref::<u64>() {
          Some(n) => Arc::new(values::List::new(vec![
            (n + 1).to_value(),
            (n + 2).to_value()])),
          _ => panic!("BAD TYPE")
        }
      });
      rule!(test_pred(y, (vec![2;2]), z) <= test_pred((3), [_], x), {
        let y, z = {succs([x])}
      });
      fact!(test_pred(3, vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred(y, (vec![2;2]), z)).unwrap(),
               vec![vec![17.to_value(), 18.to_value()]]);
    Ok(())
  })
}

#[test]
pub fn where_iter() {
  single(&|holmes : &mut Holmes| {
    try!(holmes_exec!(holmes, {
      predicate!(test_pred(uint64, bytes, uint64));
      func!(let succs : [uint64] -> [uint64] = |v : Arc<Value>| {
        match v.get().downcast_ref::<u64>() {
          Some(n) => Arc::new(values::List::new(vec![
            (n + 1).to_value(),
            (n + 2).to_value()])),
          _ => panic!("BAD TYPE")
        }
      });
      rule!(test_pred((4), (vec![2;2]), y) <= test_pred((3), [_], x), {
        let [y] = {succs([x])}
      });
      fact!(test_pred(3, vec![0;1], 16))
    }));
    assert_eq!(query!(holmes, test_pred([_], (vec![2;2]), x)).unwrap(),
               vec![vec![17.to_value()], vec![18.to_value()]]);
    Ok(())
  })
}
