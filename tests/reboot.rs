use common::*;

#[test]
fn simple() {
    multi(&[&|_holmes: &mut Holmes| Ok(()),
            &|_holmes: &mut Holmes| Ok(())])
}

#[test]
fn pred_compat() {
    multi(&[&|holmes: &mut Holmes| { holmes_exec!(holmes, {
        predicate!(test_pred(bytes, uint64))
    })},
             &|holmes: &mut Holmes| { holmes_exec!(holmes, {
        predicate!(test_pred(bytes, uint64))
    })}])
}

#[test]
fn pred_incompat() {
    multi(&[&|holmes: &mut Holmes| { holmes_exec!(holmes, {
        predicate!(test_pred(bytes, uint64))
    })},
             &|holmes: &mut Holmes| { holmes_exec!(holmes, {
        should_fail(predicate!(test_pred(bytes, uint64, uint64)))
    })}])
}

#[test]
fn fact_preserve() {
    multi(&[&|holmes: &mut Holmes| { holmes_exec!(holmes, {
        predicate!(test_pred(string, uint64));
        fact!(test_pred("foo", 7))
    })},
             &|holmes: &mut Holmes| {
        assert_eq!(query!(holmes, test_pred(("foo"), x)).unwrap(), vec![vec![7.to_value()]]);
        Ok(())
    }])
}
