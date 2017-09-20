#[macro_use]
extern crate holmes;
use holmes::simple::*;

// Ensures that rules will wake up both rules declared before and after them
#[test]
pub fn reorder() {
    single(&|holmes: &mut Engine, core: &mut Core| {
        holmes_exec!(holmes, {
            predicate!(foo(uint64));
            rule!(f12: foo((2)) <= foo((1)));
            rule!(f01: foo((1)) <= foo((0)));
            rule!(f23: foo((3)) <= foo((2)));
            fact!(foo(0))
        })?;

        core.run(holmes.quiesce()).unwrap();

        assert_eq!(query!(holmes, foo((3)))?.len(), 1);
        Ok(())
    })
}
