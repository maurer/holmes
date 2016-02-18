//! This module provides extensible, dynamically typed persistable values for
//! use in the Holmes language and postgres db.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// `HashTO` is a Trait Object-safe version of the Hash trait, using a
/// reference to a `Hasher` rather than a polymorphic one in order to allow
/// construction of trait objects.
///
/// `HashTO` is implemented automatically for any type implementing `Hash`,
/// allowing Trait Objects to implement `Hash`. For example:
///
/// ```
/// use holmes::pg::dyn::HashTO;
/// use std::hash::{Hash, Hasher};
/// trait Foo : HashTO {}
/// #[derive(Hash)]
/// struct Bar;
/// impl Foo for Bar {}
/// impl Hash for Foo {
///   fn hash<H : Hasher>(&self, hasher : &mut H) {
///     self.hash_to(hasher)
///   }
/// }
/// ```
///
/// This trick allows `Value` and `Type` trait objects to be used as keys in
/// maps by having types implementing the `ValueT` or `TypeT` interface derive
/// `Hash`.
pub trait HashTO {
  /// `hash_to` captures the same functionality as `Hash`'s `hash()`, but in
  /// a trait object safe way.
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

impl <T : Hash> HashTO for T {
  fn hash_to(&self, h : &mut Hasher) {
    self.hash(&mut HashProxy { hasher : h })
  }
}

/// Represents the type of a dynamic value as a threadsafe trait object.
pub type Type = Arc<self::types::TypeT>;
/// Represents a dynamic value as a threadsafe trait object.
pub type Value = Arc<self::values::ValueT>;

pub mod types {
  //! This module defines the trait new types must implement, along with
  //! several core types to avoid the need to rewrite basic types every time.
  //! It is heavily codependent on the `values` module.
  use super::values;
  use super::super::RowIter;
  use std::any::Any;
  use std::fmt;
  use std::sync::Arc;
  use std::hash::{Hash, Hasher};
  use super::HashTO;
  use super::Type;
  use super::Value;

  /// The TypeT trait defines the interface a new Holmes type must implement
  /// to be registered.
  pub trait TypeT : Sync + Send + HashTO + Any {
    /// For a registered type, name() will provide the way to add it to a
    /// predicate, and the thing to pattern match against when loading from the
    /// db.
    /// None will be returned if the type is anonymous, such as a tuple or
    /// list.
    fn name(&self) -> Option<&'static str>;
    /// Takes in an iterator over the row, then attempts to read a value of the
    /// type specified.
    fn extract(&self, &mut RowIter) -> Value;
    /// Generates the database representation of the fields required. For
    /// example,
    /// for a bitvector this would be
    /// ```
    /// vec!["bytea".to_string(), "int64".to_string()]
    /// ```
    /// or similar, depending on how you chose to construct the size.
    fn repr(&self) -> Vec<::std::string::String>;
    /// Returns a dynamic representation of the trait object.
    ///
    /// Trait objects cannot be cast to other trait objects, even if they
    /// implement the other trait necessarily. In order to access functions
    /// under the `Any` trait, I have the non-trait-object implementation
    /// provide access to its &Any representation.
    fn inner(&self) -> &Any;
    /// Check equality
    ///
    /// Similar to `inner`, `inner_eq` exports a `PartialEq` instance from
    /// the underlying type.
    fn inner_eq(&self, &TypeT) -> bool;
  }

  impl Hash for TypeT {
    fn hash<H : Hasher>(&self, hasher : &mut H) {
      self.hash_to(hasher)
    }
  }

  impl fmt::Debug for TypeT {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
      write!(f, "[Name: {:?}, Repr: {:?}]", self.name(), self.repr())
    }
  }

  impl Eq for TypeT {}
  impl PartialEq for TypeT {
    fn eq(&self, t : &TypeT) -> bool {
      self.inner_eq(t)
    }
  }

  /// A tuple of other `Type`s
  /// This type is anonymous.
  #[derive(Debug,Clone,Hash)]
  pub struct Tuple {
    elements : Vec<Type>
  }

  impl Tuple {
    /// Construct a new tuple from a vector of other types
    pub fn new(elems : Vec<Type>) -> Arc<Self> {
      Arc::new(Tuple { elements : elems })
    }
  }

  impl TypeT for Tuple {
    fn name(&self) -> Option<&'static str> {
      None
    }
    fn extract(&self, rows : &mut RowIter) -> Value {
      values::Tuple::new(self.elements.iter().map(|elem| {elem.extract(rows)}).collect())
    }
    fn repr(&self) -> Vec<::std::string::String> {
      self.elements.iter().flat_map(|elem| {elem.repr()}).collect()
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &TypeT) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(tup) => self.elements == tup.elements,
        // If the target is not a tuple, our types are not equal.
        None => false
      }
    }
  }

  /// A list of another `Type`
  /// This type is anonymous.
  #[derive(Debug,Clone,Hash)]
  pub struct List {
    elem : Type
  }

  impl List {
    /// Constructs a list type of the provided element type
    pub fn new(elem : Type) -> Arc<Self> {
      Arc::new(List { elem : elem })
    }
  }

  impl TypeT for List {
    fn name(&self) -> Option<&'static str> {
      None
    }
    fn extract(&self, _rows : &mut RowIter) -> Value {
      panic!("List support disabled, will be re-enabled via arrays maybe")
    }
    fn repr(&self) -> Vec<::std::string::String> {
      panic!("List support disabled, will be re-enabled via arrays maybe")
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &TypeT) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(ref tup) => self.elem == tup.elem.clone(),
        // If the target is not a tuple, our types are not equal.
        None => false
      }
    }
  }

  /// Provides a list of provided named types for use by the database
  /// as a default set of types.
  pub fn default_types() -> Vec<Type> {
    vec![Arc::new(UInt64), Arc::new(String), Arc::new(Bytes), Arc::new(Bool)]
  }

  /// Boolean type
  #[derive(Debug,Clone,Hash)]
  pub struct Bool;
  impl TypeT for Bool {
    fn name(&self) -> Option<&'static str> {
      Some("bool")
    }
    fn extract(&self, rows : &mut RowIter) -> Value {
      values::Bool::new(rows.next().unwrap())
    }
    fn repr(&self) -> Vec<::std::string::String> {
      vec!["bool".to_string()]
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &TypeT) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(_) => true,
        _ => false
      }
    }
  }

  /// Unsigned 64-bit int type
  #[derive(Debug,Clone,Hash)]
  pub struct UInt64;

  impl TypeT for UInt64 {
    fn name(&self) -> Option<&'static str> {
      Some("uint64")
    }
    fn extract(&self, rows : &mut RowIter) -> Value {
      let x : i64 = rows.next().unwrap();
      values::UInt64::new(x as u64)
    }
    fn repr(&self) -> Vec<::std::string::String> {
      vec!["int8".to_string()]
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &TypeT) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(_) => true,
        _ => false
      }
    }
  }

  /// `String` type
  /// Use this for text. If you want to store a buffer, use `Bytes` instead.
  #[derive(Debug,Clone,Hash)]
  pub struct String;

  impl TypeT for String {
    fn name(&self) -> Option<&'static str> {
      Some("string")
    }
    fn extract(&self, rows : &mut RowIter) -> Value {
      values::String::new(rows.next().unwrap())
    }
    fn repr(&self) -> Vec<::std::string::String> {
      vec!["varchar".to_string()]
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &TypeT) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(_) => true,
        _ => false
      }
    }
  }

  /// `Bytes` is for storing raw data
  /// If you want to store text, use the `String` type.
  #[derive(Debug,Clone,Hash)]
  pub struct Bytes;

  impl TypeT for Bytes {
    fn name(&self) -> Option<&'static str> {
      Some("bytes")
    }
    fn extract(&self, rows : &mut RowIter) -> Value {
      values::Bytes::new(rows.next().unwrap())
    }
    fn repr(&self) -> Vec<::std::string::String> {
      vec!["bytea".to_string()]
    }
    fn inner(&self) -> &Any {
      self as &Any
    }
    fn inner_eq(&self, other : &TypeT) -> bool {
      match other.inner().downcast_ref::<Self>() {
        Some(_) => true,
        _ => false
      }
    }
  }

}

pub mod values {
  //! This module defines the trait new values must implement, along with
  //! several core values to instantiate the basic types provided in `types`.
  //! It is heavily codependent on the `values` module.
  use postgres::types::ToSql;
  use std::any::Any;
  use super::HashTO;
  use std::sync::Arc;
  use std::hash::{Hash, Hasher};
  use std::fmt;
  use super::Type;
  use super::Value;
  use super::types;
  use std::cmp::Ordering;

  /// This trait defines the interface any value must implement in order to be
  /// used in the Holmes language.
  pub trait ValueT : Sync + Send + HashTO + fmt::Debug + Any {
    /// Returns the type of the value
    /// This is needed if to do type checking, or tuple values
    fn type_(&self) -> Type;
    /// Get a rust dynamic type version of the value contained.
    ///
    /// Since Holmes is a typed language, the user of this function should
    /// know what to expect and be able to cast from Any into it.
    fn get(&self) -> &Any;
    /// Converts the value into a list of ToSql trait objects to allow for
    /// insertion into the database via a prepared query
    fn to_sql(&self) -> Vec<&ToSql>;
    /// Returns a dynamic representation of the trait object.
    ///
    /// Trait objects cannot be cast to other trait objects, even if they
    /// implement the other trait necessarily. In order to access functions
    /// under the `Any` trait, I have the non-trait-object implementation
    /// provide access to its &Any representation.
    fn inner(&self) -> &Any;
    /// Check equality
    ///
    /// Similar to `inner`, `inner_eq` exports a `PartialEq` instance from
    /// the underlying type.
    fn inner_eq(&self, other : &ValueT) -> bool;
    /// Check order
    ///
    /// Similar to `inner`, `inner_ord` exports an `Ord` instance form
    /// the underlying type
    fn inner_ord(&self, &ValueT) -> Option<Ordering>;
  }

  impl Hash for ValueT {
    fn hash<H : Hasher>(&self, hasher : &mut H) {
      self.hash_to(hasher)
    }
  }

  /// Represents the transformability of some type into a dynamic value.
  ///
  /// This is useful both as an easy way to turn Rust values into Holmes
  /// values, and to allow for the use of literals in the macro DSL system.
  pub trait ToValue {
    /// Converts a rust native type into a Holmes `Value`
    fn to_value(self) -> Value;
  }

  impl Eq for ValueT {}
  impl PartialEq for ValueT {
    fn eq(&self, other : &ValueT) -> bool {
      self.inner_eq(other)
    }
  }

  impl PartialOrd for ValueT {
    fn partial_cmp(&self, other : &ValueT) -> Option<Ordering> {
      self.inner_ord(other)
    }
  }

  /// A list of samely typed values.
  #[derive(Debug,Clone,PartialEq,Hash,PartialOrd,Eq)]
  pub struct List {
    elements : Vec<Value>,
  }

  impl ValueT for List {
    fn type_(&self) -> Type {
      match self.elements.first() {
        Some(e) => types::List::new(e.type_()),
        //TODO have some kind of poly type to default to? Equal to everything?
        None => types::List::new(Arc::new(types::UInt64))
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
    fn inner_eq(&self, other : &ValueT) -> bool {
      let other_typed : &List = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
    fn inner_ord(&self, other : &ValueT) -> Option<Ordering> {
      other.inner().downcast_ref::<Self>().and_then(|other_typed|{
        self.partial_cmp(other_typed)
      })
    }
  }

  impl List {
    /// Create a dynamic `List` value from a list of `Value`s
    pub fn new(elements : Vec<Value>) -> Arc<Self> {
      Arc::new(List { elements : elements })
    }
  }


  /// A tuple of potentially differently typed values.
  #[derive(Debug,Clone,PartialEq,PartialOrd,Hash)]
  pub struct Tuple {
    elements : Vec<Value>,
  }

  impl ValueT for Tuple {
    fn type_(&self) -> Type {
      types::Tuple::new(self.elements.iter().map(|val| {val.type_()}).collect())
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
    fn inner_eq(&self, other : &ValueT) -> bool {
      let other_typed : &Tuple = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
    fn inner_ord(&self, other : &ValueT) -> Option<Ordering> {
      other.inner().downcast_ref::<Self>().and_then(|other_typed|{
        self.partial_cmp(other_typed)
      })
    }
  }

  impl Tuple {
    /// Create a dynamic `Tuple` value from a vector of its components.
    pub fn new(elements : Vec<Value>) -> Arc<Self> {
      Arc::new(Tuple { elements : elements })
    }
  }

  /// Holds a boolean
  #[derive(Debug,PartialEq,PartialOrd,Hash)]
  pub struct Bool {
    val : bool
  }

  impl Bool {
    /// Creates a new boolean Holmes value
    pub fn new(b : bool) -> Arc<Self> {
      Arc::new(Bool { val : b })
    }
  }

  impl ValueT for Bool {
    fn type_(&self) -> Type {
      Arc::new(types::Bool)
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
    fn inner_eq(&self, other : &ValueT) -> bool {
      let other_typed : &Bool = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
    fn inner_ord(&self, other : &ValueT) -> Option<Ordering> {
      other.inner().downcast_ref::<Self>().and_then(|other_typed|{
        self.partial_cmp(other_typed)
      })
    }
  }


  impl ToValue for bool {
    fn to_value(self) -> Value {
      Bool::new(self)
    }
  }

  impl ToValue for u64 {
    fn to_value(self) -> Value {
      UInt64::new(self)
    }
  }
  impl ToValue for ::std::string::String {
    fn to_value(self) -> Value {
      String::new(self)
    }
  }
  impl ToValue for Vec<u8> {
    fn to_value(self) -> Value {
      Bytes::new(self)
    }
  }
  impl <T : ToValue> ToValue for Vec<T> {
    fn to_value(self) -> Value {
      List::new(self.into_iter().map(|x|{x.to_value()}).collect())
    }
  }
  macro_rules! to_value_tuple {
    ($($slot:ident),*) => {
      #[allow(non_snake_case)]
      impl <$($slot : ToValue),*> ToValue for ($($slot),*) {
        fn to_value(self) -> Value {
          let ($($slot),*) = self;
          Tuple::new(vec![$($slot.to_value()),*])
        }
      }
    };
  }
  to_value_tuple!(A, B);
  to_value_tuple!(A, B, C);
  to_value_tuple!(A, B, C, D);
  to_value_tuple!(A, B, C, D, E);
  to_value_tuple!(A, B, C, D, E, F);
  to_value_tuple!(A, B, C, D, E, F, G);
  to_value_tuple!(A, B, C, D, E, F, G, H);
  impl ToValue for &'static str {
    fn to_value(self) -> Value {
      String::new(self.to_string())
    }
  }

  /// Holds an unsigned 64-bit int
  #[derive(Debug,PartialEq,PartialOrd,Hash)]
  pub struct UInt64 {
    val : u64,
    sql : i64
  }

  impl ValueT for UInt64 {
    fn type_(&self) -> Type {
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
    fn inner_eq(&self, other : &ValueT) -> bool {
      let other_typed : &UInt64 = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
    fn inner_ord(&self, other : &ValueT) -> Option<Ordering> {
      other.inner().downcast_ref::<Self>().and_then(|other_typed|{
        self.partial_cmp(other_typed)
      })
    }
  }

  impl UInt64 {
    /// Creates Holmes value holding an unsigned 64-bit integer
    pub fn new(val : u64) -> Arc<Self> {
      Arc::new(UInt64 { val : val, sql : val as i64 })
    }
  }

  /// Holds text
  #[derive(Debug,PartialEq,PartialOrd,Hash)]
  pub struct String {
    val : ::std::string::String,
  }

  impl ValueT for String {
    fn type_(&self) -> Type {
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
    fn inner_eq(&self, other : &ValueT) -> bool {
      let other_typed : &String = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
    fn inner_ord(&self, other : &ValueT) -> Option<Ordering> {
      other.inner().downcast_ref::<Self>().and_then(|other_typed|{
        self.partial_cmp(other_typed)
      })
    }
  }

  impl String {
    /// Creates a Holmes value holding a `String`
    pub fn new(val : ::std::string::String) -> Arc<Self> {
      Arc::new(String { val : val })
    }
  }

  /// Holds raw data
  #[derive(Debug,PartialEq,PartialOrd,Hash)]
  pub struct Bytes {
    val : Vec<u8>,
  }

  impl ValueT for Bytes {
    fn type_(&self) -> Type {
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
    fn inner_eq(&self, other : &ValueT) -> bool {
      let other_typed : &Bytes = match other.inner().downcast_ref::<Self>() {
        Some(x) => x,
        None => return false
      };
      self == other_typed
    }
    fn inner_ord(&self, other : &ValueT) -> Option<Ordering> {
      other.inner().downcast_ref::<Self>().and_then(|other_typed|{
        self.partial_cmp(other_typed)
      })
    }
  }

  impl Bytes {
    /// Creates a new Holmes value holding raw data.
    pub fn new(val : Vec<u8>) -> Arc<Self> {
      Arc::new(Bytes { val : val })
    }
  }
}
