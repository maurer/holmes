#[macro_use]
extern crate holmes;
use holmes::simple::*;

#[test]
pub fn new_predicate_named_field() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string], bytes, uint64))
        })
    })
}

#[test]
pub fn new_predicate_doc_field() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string "This is the first element"], bytes, uint64))
        })
    })
}

#[test]
pub fn new_predicate_doc_all() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string "This is the first element"],
                                  bytes, uint64)
                       : "This is a test predicate")
        })
    })
}

#[test]
pub fn predicate_roundtrip() {
    single(&|holmes: &mut Engine, _| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string "This is the first element"],
                                 bytes, uint64)
                       : "This is a test predicate")
        })?;
        let pred = holmes.get_predicate("test_pred")?.unwrap();
        assert_eq!(pred.description.as_ref().unwrap(), "This is a test predicate");
        assert_eq!(pred.fields[0].name.as_ref().unwrap(), "first");
        assert_eq!(pred.fields[0].description.as_ref().unwrap(), "This is the first element");
        Ok(())
    })
}

#[test]
pub fn named_field_rule() {
    single(&|holmes: &mut Engine, core: &mut Core| {
        holmes_exec!(holmes, {
            predicate!(test_pred([foo string],
                                 uint64,
                                 [bar string]));
            predicate!(out_pred(string));
            rule!(test_to_out: out_pred(x) <= test_pred {bar = x, foo = ("woo")});
            fact!(test_pred("woo", 3, "Right"));
            fact!(test_pred("wow", 4, "Wrong"))
        })?;

        core.run(holmes.quiesce()).unwrap();

        let ans = query!(holmes, out_pred(x))?;
        assert_eq!(ans, vec![["Right".to_value()]]);

        Ok(())
    })
}
