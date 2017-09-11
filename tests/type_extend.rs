#[macro_use]
extern crate holmes;
extern crate postgres;
use holmes::simple::*;

use std::any::Any;
use postgres::types::ToSql;
use holmes::pg::RowIter;
use holmes::pg::dyn::values::ValueT;
use holmes::pg::dyn::types::TypeT;
use std::fmt;

#[derive(Debug, Clone, Hash)]
struct BoolType;
impl TypeT for BoolType {
    fn name(&self) -> Option<&'static str> {
        Some("bool2")
    }
    fn extract(&self, rows: &mut RowIter) -> Option<Value> {
        rows.next().map(|b| Arc::new(BoolValue::new(b)) as Value)
    }
    fn repr(&self) -> &'static str {
        "bool"
    }
    fn inner(&self) -> &Any {
        self as &Any
    }
    fn inner_eq(&self, other: &TypeT) -> bool {
        match other.inner().downcast_ref::<Self>() {
            Some(_) => true,
            _ => false,
        }
    }
    fn large(&self) -> bool {
        false
    }
}

#[derive(Debug, PartialEq, PartialOrd, Hash)]
pub struct BoolValue {
    val: bool,
}

impl ToValue for BoolValue {
    fn to_value(self) -> Value {
        Arc::new(BoolValue::new(self.val))
    }
}

impl fmt::Display for BoolValue {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        write!(fmt, "{}", self.val)
    }
}

impl ValueT for BoolValue {
    fn type_(&self) -> Type {
        Arc::new(BoolType)
    }
    fn get(&self) -> &Any {
        &self.val as &Any
    }
    fn to_sql(&self) -> Vec<&ToSql> {
        vec![&self.val]
    }
    fn inner(&self) -> &Any {
        self as &Any
    }
    fn inner_eq(&self, other: &ValueT) -> bool {
        match other.inner().downcast_ref::<Self>() {
            Some(x) => self == x,
            _ => false,
        }
    }
    fn inner_ord(&self, other: &ValueT) -> Option<::std::cmp::Ordering> {
        other.inner().downcast_ref::<Self>().and_then(
            |x| self.partial_cmp(x),
        )
    }
}

impl BoolValue {
    pub fn new(val: bool) -> Self {
        BoolValue { val: val }
    }
}

#[test]
pub fn add_bool() {
    single(&|holmes: &mut Engine, _| {
        try!(holmes.add_type(Arc::new(BoolType)));
        try!(predicate!(holmes, type_pred(uint64, bool2)));
        try!(fact!(holmes, type_pred(32, BoolValue::new(false))));
        try!(fact!(holmes, type_pred(42, BoolValue::new(true))));
        assert_eq!(
            try!(query!(holmes, type_pred((32), x))),
            vec![vec![BoolValue::new(false).to_value()]]
        );
        assert_eq!(
            try!(query!(holmes, type_pred((42), x))),
            vec![vec![BoolValue::new(true).to_value()]]
        );
        Ok(())
    })
}

#[test]
fn reboot() {
    multi(
        &[
            &|holmes: &mut Engine, _| holmes.add_type(Arc::new(BoolType)),
            &|_holmes: &mut Engine, _| Ok(()),
        ],
    )
}

#[test]
fn reboot_reuse() {
    multi(
        &[
            &|holmes: &mut Engine, _| {
                try!(holmes.add_type(Arc::new(BoolType)));
                try!(predicate!(holmes, type_pred(uint64, bool2)));
                fact!(holmes, type_pred(32, BoolValue::new(false)))
            },
            &|holmes: &mut Engine, _| {
                try!(holmes.add_type(Arc::new(BoolType)));
                try!(predicate!(holmes, type_pred(uint64, bool2)));
                try!(fact!(holmes, type_pred(42, BoolValue::new(true))));
                assert_eq!(
                    try!(query!(holmes, type_pred((32), x))),
                    vec![vec![BoolValue::new(false).to_value()]]
                );
                assert_eq!(
                    try!(query!(holmes, type_pred((42), x))),
                    vec![vec![BoolValue::new(true).to_value()]]
                );
                Ok(())
            },
        ],
    )
}
