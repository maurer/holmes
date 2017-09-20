#[macro_use]
extern crate holmes;
use holmes::simple::*;

#[test]
pub fn one_step() {
    single(&|holmes: &mut Engine, core: &mut Core| {
        try!(holmes_exec!(holmes, {
      predicate!(test_pred(string, bytes, uint64));
      fact!(test_pred("foo", vec![3u8;3], 7));
      rule!(test_forward: test_pred(("bar"), (vec![2u8;2]), x) <= test_pred(("foo"), [_], x))
    }));
        core.run(holmes.quiesce()).unwrap();
        assert_eq!(query!(holmes, test_pred(("bar"), [_], x)).unwrap(),
               vec![vec![7.to_value()]]);
        Ok(())
    })
}

#[test]
pub fn closure() {
    single(&|holmes: &mut Engine, core: &mut Core| {
        try!(holmes_exec!(holmes, {
      predicate!(reaches(string, string));
      fact!(reaches("foo", "bar"));
      fact!(reaches("bar", "baz"));
      fact!(reaches("baz", "bang"));
      rule!(reaches_trans: reaches(src, dst) <= reaches(src, mid) & reaches(mid, dst))
    }));
        core.run(holmes.quiesce()).unwrap();
        let ans = try!(query!(holmes, reaches(("foo"), tgt)));
        assert_eq!(ans, vec![["bar".to_value()], ["baz".to_value()], ["bang".to_value()]]);
        Ok(())
    })
}
