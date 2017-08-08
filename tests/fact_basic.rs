#[macro_use]
extern crate holmes;
use holmes::simple::*;

#[test]
pub fn new_fact_basic() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred(string, bytes, uint64));
            fact!(test_pred("foo", vec![3u8, 4u8, 5u8], 7))
        })
    })
}

#[test]
pub fn new_fact_type_err() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred(string, bytes, uint64));
            should_fail(fact!(test_pred(7, vec![3u8, 4u8, 5u8], 7)))
        })
    })
}

#[test]
pub fn new_fact_echo() {
    single(&|holmes: &mut Engine, _| {
        try!(holmes_exec!(holmes, {
            predicate!(test_pred(string, bytes, uint64));
            fact!(test_pred("foo", vec![3u8, 3u8], 7))
        }));
        assert_eq!(query!(holmes,
                       test_pred(("foo"), [_], x)).unwrap(),
               vec![vec![7.to_value()]]);
        Ok(())
    })
}

#[test]
pub fn two_strings() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred(string, string));
            fact!(test_pred("foo", "bar"))
        })
    })
}
