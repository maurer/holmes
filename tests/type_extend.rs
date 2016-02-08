use common::*;
use std::any::Any;
use postgres::types::ToSql;
use holmes::db_types::RowIter;

#[derive(Debug,Clone,Hash)]
struct BoolType;
impl Type for BoolType {
  fn name(&self) -> Option<&'static str> {
    Some("bool")
  }
  fn extract(&self, rows : &mut RowIter) -> Arc<Value> {
    Arc::new(BoolValue::new(rows.next().unwrap()))
  }
  fn repr(&self) -> Vec<String> {
    vec!["bool".to_string()]
  }
  fn inner(&self) -> &Any {
    self as &Any
  }
  fn inner_eq(&self, other : &Type) -> bool {
    match other.inner().downcast_ref::<Self>() {
      Some(_) => true,
      _ => false
    }
  }
}

#[derive(Debug,PartialEq,Hash)]
pub struct BoolValue {
  val : bool
}

impl ToValue for BoolValue {
  fn to_value(self) -> Arc<Value> {
    Arc::new(BoolValue::new(self.val))
  }
}

impl Value for BoolValue {
  fn type_(&self) -> Arc<Type> {
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
  fn inner_eq(&self, other : &Value) -> bool {
    match other.inner().downcast_ref::<Self>() {
      Some(x) => self == x,
      _ => false
    }
  }
}

impl BoolValue {
  pub fn new(val : bool) -> Self {
    BoolValue { val : val }
  }
}

#[test]
pub fn add_bool() {
  single(&|holmes : &mut Holmes| {
    assert!(holmes.add_type(Arc::new(BoolType)));
    try!(predicate!(holmes, type_pred(uint64, bool)));
    try!(fact!(holmes, type_pred(32, BoolValue::new(false))));
    try!(fact!(holmes, type_pred(42, BoolValue::new(true))));
    assert_eq!(try!(query!(holmes, type_pred((32), x))),
               vec![vec![BoolValue::new(false).to_value()]]);
    assert_eq!(try!(query!(holmes, type_pred((42), x))),
               vec![vec![BoolValue::new(true).to_value()]]);
    Ok(())
  })
}
