#[macro_use]
extern crate holmes;
use holmes::simple::*;

#[test]
pub fn turn_on() {
    single(&|_holmes: &mut Engine, _| Ok(()))
}

#[test]
pub fn macro_check() {
    single(&|holmes: &mut Engine, _| holmes_exec!(holmes, {}))
}
