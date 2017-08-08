#[macro_use]
extern crate holmes;
use holmes::simple::*;

#[test]
pub fn new_predicate_basic() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred(string, bytes, uint64))
        })
    })
}

#[test]
pub fn double_register_incompat() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred(string, bytes, uint64));
            should_fail(predicate!(test_pred(string, string, string)))
        })
    })
}

#[test]
pub fn double_register_compat() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred(string, bytes, uint64));
            predicate!(test_pred(string, bytes, uint64))
        })
    })
}
