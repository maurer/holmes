use std::hash::{Hash, Hasher};

pub trait HashTO {
  fn hash_to(&self, &mut Hasher);
}

struct HashProxy<'a> {
  pub hasher : &'a mut Hasher
}

impl <'a> Hasher for HashProxy<'a> {
  fn finish(&self) -> u64 {
    self.hasher.finish()
  }
  fn write(&mut self, bytes : &[u8]) {
    self.hasher.write(bytes)
  }
}

impl <T : Hash + Sized> HashTO for T {
  fn hash_to(&self, h : &mut Hasher) {
    //TODO: all data structures will be terrible until this is fixed
    self.hash(&mut HashProxy { hasher : h })
  }
}

pub mod types {
  use super::values;
  use super::values::Value;
  use super::super::RowIter;
  use std::any::Any;
  use std::fmt;
  use std::sync::Arc;
  use std::hash::{Hash, Hasher};
  use super::HashTO;

  pub trait Type : Sync + Send + HashTO + Any {
    // For a registered type, name() will provide the way to add it to a
    // predicate, and the thing to pattern match against when loading from the
    // db.
    // None will be returned if the type is anonymous, such as a tuple or list.
    fn name(&self) -> Option<&'static str>;
    // Takes in an iterator over the row, then attempts to read a value.
    // Since the underlying .get() will panic, I'm not including an additional
    // error reporting path (for now)
    fn extract(&self, &mut RowIter) -> Arc<Value>;
    // Generates the database representation of the fields required. For example,
    // for a bitvector this would be
    // vec!["bytea".to_string(), "int64".to_string()]
    // or similar, depending on how you chose to construct the size
    fn repr(&self) -> Vec<::std::string::String>;
    // Escape hatch
    fn inner(&self) -> &Any;
    // Check equality
    // This should be symmetric, but we have no way to enforce it within the
    // type system of rust.
    fn inner_eq(&self, &Type) -> bool;
  }

  impl Hash for Type {
    fn hash<H : Hasher>(&self, hasher : &mut H) {
      self.hash_to(hasher)
    }
  }

  impl fmt::Debug for Type {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
      write!(f, "[Name: {:?}, Repr: {:?}]", self.name(), self.repr())
    }
  }

  impl Eq for Type {}
  impl PartialEq for Type {
    fn eq(&self, t : &Type) -> bool {
      self.inner_eq(t)
    }
  }

  #[derive(Debug,Clone,Hash)]
  pub struct Tuple {
    elements : Vec<Arc<Type>>
  }

  impl Tuple {
    pub fn new(elems : Vec<Arc<Type>>) -> Self {
      Tuple { elements : elems }
    }
  }

  impl Type for Tuple {
    fn name(&self) -> Option<&'static str> {
      None
    }
    fn extract(&self, rows : &mut RowIter) -> Arc<Value> {
      Arc::new(values::Tuple::new(self.elements.iter().map(|elem| {elem.extract(rows)}).collect()))
    }
    fn repr(&self) -> Vec<::std::string::String> {
      self.elements.iter().flat_map(|elem| {elem.repr()}).collect()
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &Type) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(tup) => self.elements == tup.elements,
        // If the target is not a tuple, our types are not equal.
        None => false
      }
    }
  }

  #[derive(Debug,Clone,Hash)]
  pub struct List {
    elem : Arc<Type>
  }

  impl List {
    pub fn new(elem : Arc<Type>) -> Self {
      List { elem : elem }
    }
  }

  impl Type for List {
    fn name(&self) -> Option<&'static str> {
      None
    }
    fn extract(&self, _rows : &mut RowIter) -> Arc<Value> {
      panic!("List support disabled, will be re-enabled via arrays maybe")
    }
    fn repr(&self) -> Vec<::std::string::String> {
      panic!("List support disabled, will be re-enabled via arrays maybe")
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &Type) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(ref tup) => self.elem == tup.elem.clone(),
        // If the target is not a tuple, our types are not equal.
        None => false
      }
    }
  }

  pub fn default_types() -> Vec<Arc<Type>> {
    vec![Arc::new(UInt64), Arc::new(String), Arc::new(Bytes)]
  }

  #[derive(Debug,Clone,Hash)]
  pub struct UInt64;

  impl Type for UInt64 {
    fn name(&self) -> Option<&'static str> {
      Some("uint64")
    }
    fn extract(&self, rows : &mut RowIter) -> Arc<Value> {
      let x : i64 = rows.next().unwrap();
      Arc::new(values::UInt64::new(x as u64))
    }
    fn repr(&self) -> Vec<::std::string::String> {
      vec!["int8".to_string()]
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

  #[derive(Debug,Clone,Hash)]
  pub struct String;

  impl Type for String {
    fn name(&self) -> Option<&'static str> {
      Some("string")
    }
    fn extract(&self, rows : &mut RowIter) -> Arc<Value> {
      Arc::new(values::String::new(rows.next().unwrap()))
    }
    fn repr(&self) -> Vec<::std::string::String> {
      vec!["varchar".to_string()]
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

  #[derive(Debug,Clone,Hash)]
  pub struct Bytes;

  impl Type for Bytes {
    fn name(&self) -> Option<&'static str> {
      Some("bytes")
    }
    fn extract(&self, rows : &mut RowIter) -> Arc<Value> {
      Arc::new(values::Bytes::new(rows.next().unwrap()))
    }
    fn repr(&self) -> Vec<::std::string::String> {
      vec!["bytea".to_string()]
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

}

pub mod values {
  use postgres::types::ToSql;
  use std::any::Any;
  use super::types;
  use super::types::Type;
  use super::HashTO;
  use std::sync::Arc;
  use std::hash::{Hash, Hasher};
  use std::fmt;

  pub trait Value : Sync + Send + HashTO + fmt::Debug + Any {
    // Returns the type, needed if you want to do type checking, or tuple values
    fn type_(&self) -> Arc<Type>;
    // Get a rust dynamic type, to be used by someone who was typed in the holmes
    // language, and so knows what they're actually getting
    fn get(&self) -> &Any;
    // Converts the value into a list of ToSql trait objects to allow for
    // insertion into the database via a prepared query
    fn to_sql(&self) -> Vec<&ToSql>;
    // Escape hatch
    fn inner(&self) -> &Any;
    // Used to test equality, to implement PartialEq
    fn inner_eq(&self, other : &Value) -> bool;
  }

  impl Hash for Value {
    fn hash<H : Hasher>(&self, hasher : &mut H) {
      self.hash_to(hasher)
    }
  }

  pub trait ToValue {
    fn to_value(self) -> Arc<Value>;
  }

  impl Eq for Value {}
  impl PartialEq for Value {
    fn eq(&self, other : &Value) -> bool {
      self.inner_eq(other)
    }
  }

  #[derive(Debug,Clone,PartialEq,Hash)]
  pub struct List {
    elements : Vec<Arc<Value>>,
  }

  impl Value for List {
    fn type_(&self) -> Arc<Type> {
      match self.elements.first() {
        Some(e) => Arc::new(types::List::new(e.type_())),
        //TODO have some kind of poly type to default to? Equal to everything?
        None => Arc::new(types::List::new(Arc::new(types::UInt64)))
      }
    }
    fn get(&self) -> &Any {
      &self.elements as &Any
    }
    fn to_sql(&self) -> Vec<&ToSql> {
      panic!("List SQL disabled")
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &Value) -> bool {
      let other_typed : &List = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
  }

  impl List {
    pub fn new(elements : Vec<Arc<Value>>) -> Self {
      List { elements : elements }
    }
  }


  #[derive(Debug,Clone,PartialEq,Hash)]
  pub struct Tuple {
    elements : Vec<Arc<Value>>,
  }

  impl Value for Tuple {
    fn type_(&self) -> Arc<Type> {
      Arc::new(types::Tuple::new(self.elements.iter().map(|val| {val.type_()}).collect()))
    }
    fn get(&self) -> &Any {
      &self.elements as &Any
    }
    fn to_sql(&self) -> Vec<&ToSql> {
      self.elements.iter().flat_map(|val| val.to_sql()).collect()
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &Value) -> bool {
      let other_typed : &Tuple = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
  }

  impl Tuple {
    pub fn new(elements : Vec<Arc<Value>>) -> Self {
      Tuple { elements : elements }
    }
  }

  #[derive(Debug,PartialEq,Hash)]
  pub struct UInt64 {
    val : u64,
    sql : i64
  }

  impl ToValue for u64 {
    fn to_value(self) -> Arc<Value> {
      Arc::new(UInt64::new(self))
    }
  }
  impl ToValue for ::std::string::String {
    fn to_value(self) -> Arc<Value> {
      Arc::new(String::new(self))
    }
  }
  impl ToValue for Vec<u8> {
    fn to_value(self) -> Arc<Value> {
      Arc::new(Bytes::new(self))
    }
  }
  impl ToValue for &'static str {
    fn to_value(self) -> Arc<Value> {
      Arc::new(String::new(self.to_string()))
    }
  }

  impl Value for UInt64 {
    fn type_(&self) -> Arc<Type> {
      Arc::new(types::UInt64)
    }
    fn get(&self) -> &Any {
      &self.val as &Any
    }
    fn to_sql(&self) -> Vec<&ToSql> {
      vec![&self.sql]
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &Value) -> bool {
      let other_typed : &UInt64 = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
  }

  impl UInt64 {
    pub fn new(val : u64) -> Self {
      UInt64 { val : val, sql : val as i64 }
    }
  }

  #[derive(Debug,PartialEq,Hash)]
  pub struct String {
    val : ::std::string::String,
  }

  impl Value for String {
    fn type_(&self) -> Arc<Type> {
      Arc::new(types::String)
    }
    fn get(&self) -> &Any {
      &self.val as &Any
    }
    fn to_sql(&self) -> Vec<&ToSql> {
      vec![&self.val as &ToSql]
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &Value) -> bool {
      let other_typed : &String = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
  }

  impl String {
    pub fn new(val : ::std::string::String) -> Self {
      String { val : val }
    }
  }

  #[derive(Debug,PartialEq,Hash)]
  pub struct Bytes {
    val : Vec<u8>,
  }

  impl Value for Bytes {
    fn type_(&self) -> Arc<Type> {
      Arc::new(types::Bytes)
    }
    fn get(&self) -> &Any {
      &self.val as &Any
    }
    fn to_sql(&self) -> Vec<&ToSql> {
      vec![&self.val as &ToSql]
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &Value) -> bool {
      let other_typed : &Bytes = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
  }

  impl Bytes {
    pub fn new(val : Vec<u8>) -> Self {
      Bytes { val : val }
    }
  }

}
