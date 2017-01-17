use common::*;

#[test]
pub fn new_predicate_named_field() {
    single(&|holmes: &mut Engine| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string], bytes, uint64))
        })
    })
}

#[test]
pub fn new_predicate_doc_field() {
    single(&|holmes: &mut Engine| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string "This is the first element"], bytes, uint64))
        })
    })
}

#[test]
pub fn new_predicate_doc_all() {
    single(&|holmes: &mut Engine| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string "This is the first element"], bytes, uint64) : "This is a test predicate")
        })
    })
}

#[test]
pub fn predicate_roundtrip() {
    single(&|holmes: &mut Engine| {
        holmes_exec!(holmes, {
            predicate!(test_pred([first string "This is the first element"], bytes, uint64) : "This is a test predicate")
        })?;
	let pred = holmes.get_predicate("test_pred")?.unwrap();
	assert_eq!(pred.description.as_ref().unwrap(), "This is a test predicate");
	assert_eq!(pred.fields[0].name.as_ref().unwrap(), "first");
	assert_eq!(pred.fields[0].description.as_ref().unwrap(), "This is the first element");
	Ok(())
    })
}
