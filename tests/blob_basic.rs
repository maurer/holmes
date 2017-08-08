#[macro_use]
extern crate holmes;
use holmes::simple::*;
use holmes::pg::dyn::values::LargeBWrap;

#[test]
pub fn insert() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred(largebytes));
            fact!(test_pred(LargeBWrap { inner: vec![3u8, 4u8, 5u8] }))
        })
    })
}

#[test]
pub fn roundtrip() {
    single(&|holmes: &mut Engine, _| {
        try!(holmes_exec!(holmes, {
            predicate!(test_pred(uint64, largebytes));
            fact!(test_pred(3, LargeBWrap { inner: vec![3u8, 3u8] }))
        }));
        assert_eq!(
            query!(holmes, test_pred((3), x)).unwrap(),
            vec![vec![LargeBWrap { inner: vec![3u8, 3u8] }.to_value()]]
        );
        Ok(())
    })
}

// The point of this test is to hit the open-file-cache to make sure it's not completely broken
#[test]
pub fn double_query() {
    single(&|holmes: &mut Engine, _| {
        try!(holmes_exec!(holmes, {
            predicate!(test_pred(uint64, largebytes));
            fact!(test_pred(3, LargeBWrap { inner: vec![3u8, 3u8] }))
        }));
        assert_eq!(
            query!(holmes, test_pred((3), x)).unwrap(),
            vec![vec![LargeBWrap { inner: vec![3u8, 3u8] }.to_value()]]
        );
        assert_eq!(
            query!(holmes, test_pred((3), x)).unwrap(),
            vec![vec![LargeBWrap { inner: vec![3u8, 3u8] }.to_value()]]
        );
        Ok(())
    })
}
