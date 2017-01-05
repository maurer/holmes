use common::*;

#[test]
pub fn turn_on() {
    single(&|_holmes: &mut Engine| Ok(()))
}

#[test]
pub fn macro_check() {
    single(&|holmes: &mut Engine| {
        holmes_exec!(holmes, {
        })
    })
}
