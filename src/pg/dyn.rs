//! This module provides extensible, dynamically typed persistable values for
//! use in the Holmes language and postgres db.
//! TODO put a demo on custom types in here

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
    pub hasher: &'a mut Hasher,
}

impl<'a> Hasher for HashProxy<'a> {
    fn finish(&self) -> u64 {
        self.hasher.finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        self.hasher.write(bytes)
    }
}

impl<T: Hash> HashTO for T {
    fn hash_to(&self, h: &mut Hasher) {
        self.hash(&mut HashProxy { hasher: h })
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

    #[macro_export]
    macro_rules! typet_inner {
      () => {
          fn inner(&self) -> &::std::any::Any {
              self as &::std::any::Any
          }
      }
  }

    #[macro_export]
    macro_rules! typet_inner_eq {
      () => {
          fn inner_eq(&self, other : &TypeT) -> bool {
              let other_self = match other.inner().downcast_ref() {
                  Some(x) => x,
                  None => return false
              };
              self == other_self
          }
      }
  }

    #[macro_export]
    macro_rules! typet_boiler {
      () => {
          typet_inner!();
          typet_inner_eq!();
          fn large(&self) -> bool {
              false
          }
      }
  }

    /// The TypeT trait defines the interface a new Holmes type must implement
    /// to be registered.
    pub trait TypeT: HashTO + Any {
        /// For a registered type, name() will provide the way to add it to a
        /// predicate, and the thing to pattern match against when loading from the
        /// db.
        /// None will be returned if the type is anonymous, such as a tuple or
        /// list.
        fn name(&self) -> Option<&'static str>;
        /// Takes in an iterator over the row, then attempts to read a value of the
        /// type specified.
        fn extract(&self, &mut RowIter) -> Option<Value>;
        /// Generates the database representation of the field required.
        fn repr(&self) -> &'static str;
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
        fn inner_eq(&self, other: &TypeT) -> bool;
        /// List of subindexes to be ignored when checking uniqueness.
        /// Intended to be used to ignore large payloads in favor of hashes
        fn large(&self) -> bool;
    }

    impl Hash for TypeT {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
            self.hash_to(hasher)
        }
    }

    impl fmt::Debug for TypeT {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "[Name: {:?}, Repr: {:?}]", self.name(), self.repr())
        }
    }

    impl Eq for TypeT {}
    impl PartialEq for TypeT {
        fn eq(&self, t: &TypeT) -> bool {
            self.inner_eq(t)
        }
    }

    /// The Trap type is used to represent types not yet present in the program,
    /// but present in the database. Most forms of interaction with a trap type
    /// will take down the program.
    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct Trap;
    impl TypeT for Trap {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            Some("trap")
        }
        fn extract(&self, _rows: &mut RowIter) -> Option<Value> {
            panic!("Tried to extract from a trap")
        }
        fn repr(&self) -> &'static str {
            "void"
        }
    }
    impl Trap {
        /// Instantiates the Trap type
        pub fn new() -> Arc<Self> {
            Arc::new(Trap)
        }
    }

    /// A tuple of other `Type`s
    /// This type is anonymous.
    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct Tuple {
        elements: Vec<Type>,
    }

    impl Tuple {
        /// Construct a new tuple from a vector of other types
        pub fn new(elems: Vec<Type>) -> Arc<Self> {
            Arc::new(Tuple { elements: elems })
        }
    }

    impl TypeT for Tuple {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            None
        }
        fn extract(&self, _rows: &mut RowIter) -> Option<Value> {
            panic!("Tuples cannot be extracted")
        }
        fn repr(&self) -> &'static str {
            panic!("Tuples cannot be represented")
        }
    }

    /// A list of another `Type`
    /// This type is anonymous.
    #[derive(Debug, Clone, Hash)]
    pub struct List {
        elem: Type,
    }

    impl List {
        /// Constructs a list type of the provided element type
        pub fn new(elem: Type) -> Arc<Self> {
            Arc::new(List { elem: elem })
        }
    }

    // PartialEq won't derive right
    impl PartialEq for List {
        fn eq(&self, other: &Self) -> bool {
            *self.elem == *other.elem
        }
    }

    impl TypeT for List {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            None
        }
        fn extract(&self, _rows: &mut RowIter) -> Option<Value> {
            panic!("Cannot extract a list")
        }
        fn repr(&self) -> &'static str {
            panic!("Cannot represent a list")
        }
    }

    /// Provides a list of provided named types for use by the database
    /// as a default set of types.
    pub fn default_types() -> Vec<Type> {
        vec![
            Arc::new(UInt64),
            Arc::new(String),
            Arc::new(Bytes),
            Arc::new(LargeBytes),
            Arc::new(LargeString),
            Arc::new(Bool),
        ]
    }

    /// Boolean type
    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct Bool;
    impl TypeT for Bool {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            Some("bool")
        }
        fn extract(&self, rows: &mut RowIter) -> Option<Value> {
            rows.next().map(|b| values::Bool::new(b) as Value)
        }
        fn repr(&self) -> &'static str {
            "bool"
        }
    }

    /// Unsigned 64-bit int type
    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct UInt64;

    impl TypeT for UInt64 {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            Some("uint64")
        }
        fn extract(&self, rows: &mut RowIter) -> Option<Value> {
            rows.next().map(
                |x: i64| values::UInt64::new(x as u64) as Value,
            )
        }
        fn repr(&self) -> &'static str {
            "int8"
        }
    }

    /// `String` type
    /// Use this for text. If you want to store a buffer, use `Bytes` instead.
    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct String;

    impl TypeT for String {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            Some("string")
        }
        fn extract(&self, rows: &mut RowIter) -> Option<Value> {
            rows.next().map(|s| values::String::new(s) as Value)
        }
        fn repr(&self) -> &'static str {
            "varchar"
        }
    }

    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct LargeString;

    impl TypeT for LargeString {
        typet_inner!();
        typet_inner_eq!();
        fn large(&self) -> bool {
            true
        }
        fn name(&self) -> Option<&'static str> {
            Some("largestring")
        }
        fn extract(&self, rows: &mut RowIter) -> Option<Value> {
            rows.next().map(|s| values::String::new(s) as Value)
        }
        fn repr(&self) -> &'static str {
            "varchar"
        }
    }

    /// `Bytes` is for storing raw data
    /// If you want to store text, use the `String` type.
    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct Bytes;

    impl TypeT for Bytes {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            Some("bytes")
        }
        fn extract(&self, rows: &mut RowIter) -> Option<Value> {
            rows.next().map(|b| values::Bytes::new(b) as Value)
        }
        fn repr(&self) -> &'static str {
            "bytea"
        }
    }

    /// `LargeBytes` is for storing raw data which should not be considered
    /// in uniqueness checks
    /// If you want to store text, use the `String` type.
    #[derive(Debug, Clone, Hash, PartialEq)]
    pub struct LargeBytes;

    impl TypeT for LargeBytes {
        typet_boiler!();
        fn name(&self) -> Option<&'static str> {
            Some("largebytes")
        }
        fn extract(&self, rows: &mut RowIter) -> Option<Value> {
            rows.next().map(|v: ::std::string::String| {
                values::LargeBytes::from_hash(&v) as Value
            })
        }
        fn repr(&self) -> &'static str {
            "char(64)"
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

    #[macro_export]
    macro_rules! valuet_boiler {
      () => {
        fn inner(&self) -> &::std::any::Any {
          self as &::std::any::Any
        }
        fn inner_eq(&self, other : &ValueT) -> bool {
          let other_typed = match other.inner().downcast_ref::<Self>() {
            Some(x) => x,
            None => return false
          };
          self == other_typed
        }
        fn inner_ord(&self, other : &ValueT) -> Option<::std::cmp::Ordering> {
          other.inner().downcast_ref::<Self>().and_then(|other_typed|{
            self.partial_cmp(other_typed)
          })
        }
      }
  }

    /// This trait defines the interface any value must implement in order to be
    /// used in the Holmes language.
    pub trait ValueT: HashTO + fmt::Debug + fmt::Display + Any {
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
        fn inner_eq(&self, other: &ValueT) -> bool;
        /// Check order
        ///
        /// Similar to `inner`, `inner_ord` exports an `Ord` instance form
        /// the underlying type
        fn inner_ord(&self, &ValueT) -> Option<Ordering>;
    }

    impl Hash for ValueT {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
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
        fn eq(&self, other: &ValueT) -> bool {
            self.inner_eq(other)
        }
    }

    impl Ord for ValueT {
        fn cmp(&self, other: &ValueT) -> Ordering {
            self.inner_ord(other).unwrap()
        }
    }

    impl PartialOrd for ValueT {
        fn partial_cmp(&self, other: &ValueT) -> Option<Ordering> {
            self.inner_ord(other)
        }
    }

    /// A list of samely typed values.
    #[derive(Debug, Clone, PartialEq, Hash, PartialOrd, Eq)]
    pub struct List {
        elements: Vec<Value>,
    }

    impl fmt::Display for List {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(fmt, "[")?;
            let mut first = true;
            for elem in self.elements.iter() {
                if !first {
                    write!(fmt, ", ")?;
                    first = false;
                }
                write!(fmt, "{}", elem)?;
            }
            write!(fmt, "]")
        }
    }

    impl ValueT for List {
        fn type_(&self) -> Type {
            match self.elements.first() {
                Some(e) => types::List::new(e.type_()),
                // TODO have some kind of poly type to default to? Equal to everything?
                None => types::List::new(Arc::new(types::UInt64)),
            }
        }
        fn get(&self) -> &Any {
            &self.elements as &Any
        }
        fn to_sql(&self) -> Vec<&ToSql> {
            panic!("List SQL disabled")
        }
        valuet_boiler!();
    }

    impl List {
        /// Create a dynamic `List` value from a list of `Value`s
        pub fn new(elements: Vec<Value>) -> Arc<Self> {
            Arc::new(List { elements: elements })
        }
    }


    /// A tuple of potentially differently typed values.
    #[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
    pub struct Tuple {
        elements: Vec<Value>,
    }

    impl fmt::Display for Tuple {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(fmt, "(")?;
            let mut first = true;
            for elem in self.elements.iter() {
                if !first {
                    write!(fmt, ", ")?;
                    first = false;
                }
                write!(fmt, "{}", elem)?;
            }
            write!(fmt, ")")
        }
    }


    impl ValueT for Tuple {
        fn type_(&self) -> Type {
            types::Tuple::new(self.elements.iter().map(|val| val.type_()).collect())
        }
        fn get(&self) -> &Any {
            &self.elements as &Any
        }
        fn to_sql(&self) -> Vec<&ToSql> {
            self.elements.iter().flat_map(|val| val.to_sql()).collect()
        }
        valuet_boiler!();
    }

    impl Tuple {
        /// Create a dynamic `Tuple` value from a vector of its components.
        pub fn new(elements: Vec<Value>) -> Arc<Self> {
            Arc::new(Tuple { elements: elements })
        }
    }

    /// Holds a boolean
    #[derive(Debug, PartialEq, PartialOrd, Hash)]
    pub struct Bool {
        val: bool,
    }


    impl Bool {
        /// Creates a new boolean Holmes value
        pub fn new(b: bool) -> Arc<Self> {
            Arc::new(Bool { val: b })
        }
    }

    impl fmt::Display for Bool {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(fmt, "{}", self.val)
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
        valuet_boiler!();
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
    /// Wrapper newtype pattern for buffers which are too large to be reasonably
    /// indexed or matched on.
    pub struct LargeBWrap {
        /// Wrapped value
        pub inner: Vec<u8>,
    }
    impl ToValue for LargeBWrap {
        fn to_value(self) -> Value {
            LargeBytes::new(self.inner)
        }
    }

    impl<T: ToValue> ToValue for Vec<T> {
        fn to_value(self) -> Value {
            List::new(self.into_iter().map(|x| x.to_value()).collect())
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
    #[derive(Debug, PartialEq, PartialOrd, Hash)]
    pub struct UInt64 {
        val: u64,
        sql: i64,
    }

    impl fmt::Display for UInt64 {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(fmt, "{}", self.val)
        }
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
        valuet_boiler!();
    }

    impl UInt64 {
        /// Creates Holmes value holding an unsigned 64-bit integer
        pub fn new(val: u64) -> Arc<Self> {
            Arc::new(UInt64 {
                val: val,
                sql: val as i64,
            })
        }
    }

    /// Holds text
    #[derive(Debug, PartialEq, PartialOrd, Hash)]
    pub struct String {
        val: ::std::string::String,
    }

    impl fmt::Display for String {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(fmt, "{:?}", self.val)
        }
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
        valuet_boiler!();
    }

    impl String {
        /// Creates a Holmes value holding a `String`
        pub fn new(val: ::std::string::String) -> Arc<Self> {
            Arc::new(String { val: val })
        }
    }

    /// Holds raw data
    #[derive(Debug, PartialEq, PartialOrd, Hash)]
    pub struct Bytes {
        val: Vec<u8>,
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
        valuet_boiler!();
    }

    impl fmt::Display for Bytes {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(fmt, "{:?}", self.val)
        }
    }

    impl Bytes {
        /// Creates a new Holmes value holding raw data.
        pub fn new(val: Vec<u8>) -> Arc<Self> {
            Arc::new(Bytes { val: val })
        }
    }

    use std::fs::File;
    #[derive(Debug)]
    /// Holds large raw data - if your buffer is larger than 256 bytes, you probably want to use
    /// this rather than `Bytes`
    pub struct LargeBytes {
        hash: ::std::string::String,
        fd: Rc<File>,
    }

    impl PartialEq for LargeBytes {
        fn eq(&self, rhs: &Self) -> bool {
            self.hash.eq(&rhs.hash)
        }
    }
    impl PartialOrd for LargeBytes {
        fn partial_cmp(&self, rhs: &Self) -> Option<::std::cmp::Ordering> {
            self.hash.partial_cmp(&rhs.hash)
        }
    }
    impl Hash for LargeBytes {
        fn hash<T: ::std::hash::Hasher>(&self, h: &mut T) {
            self.hash.hash(h)
        }
    }

    impl ValueT for LargeBytes {
        fn type_(&self) -> Type {
            Arc::new(types::LargeBytes)
        }
        fn get(&self) -> &Any {
            use std::borrow::Borrow;
            let file_borrow: &File = self.fd.borrow();
            file_borrow as &Any
        }
        fn to_sql(&self) -> Vec<&ToSql> {
            vec![&self.hash as &ToSql]
        }
        valuet_boiler!();
    }

    impl fmt::Display for LargeBytes {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(fmt, "(large)")
        }
    }

    impl LargeBytes {
        /// Creates a new Holmes value holding raw data.
        pub fn new(val: Vec<u8>) -> Arc<Self> {
            use sha2::*;
            use std::fs::File;
            use std::io::Write;
            use rustc_serialize::hex::ToHex;
            let mut path = match ::std::env::var("HOLMES_STORAGE") {
                Ok(dir) => ::std::path::PathBuf::from(dir),
                _ => {
                    let mut path = ::std::env::home_dir().unwrap();
                    path.push(".holmes");
                    path
                }
            };
            let mut hasher = Sha256::default();
            hasher.input(&val);
            let fname = hasher.result().to_hex();
            path.push(fname.clone());
            {
                let mut file = File::create(path.clone()).unwrap();
                file.write_all(&val).unwrap();
            }
            let file = File::open(path.clone()).unwrap();
            Arc::new(LargeBytes {
                fd: Rc::new(file),
                hash: fname,
            })
        }
        /// Generate a `LargeBytes` value from its hash if already stored.
        /// This function does not have any error handling, so it should only be used if the user
        /// is certain the value has already been stored.
        pub fn from_hash(hash: &str) -> Arc<Self> {
            let file = cached_open(hash);
            Arc::new(LargeBytes {
                fd: file,
                hash: hash.to_owned(),
            })
        }
    }
    use std::collections::HashMap;
    use std::cell::RefCell;
    use std::rc::Rc;
    thread_local! {
        pub static FILE_CACHE: RefCell<HashMap<::std::string::String, Rc<File>>> =
            RefCell::new(HashMap::new());
    }
    fn cached_open(hash: &str) -> Rc<File> {
        {
            use std::ops::DerefMut;
            FILE_CACHE.with(|cache| if cache.borrow().len() > 100 {
                // We're thrashing on file descriptors, drop the cache
                trace!("FILE_CACHE THRASHING");
                *cache.borrow_mut() = HashMap::new()
            })
        }
        let file = FILE_CACHE.with(|cache| {
            cache
                .borrow_mut()
                .entry(hash.to_owned())
                .or_insert_with(|| {
                    let mut path = match ::std::env::var("HOLMES_STORAGE") {
                        Ok(dir) => ::std::path::PathBuf::from(dir),
                        _ => {
                            let mut path = ::std::env::home_dir().unwrap();
                            path.push(".holmes");
                            path
                        }
                    };

                    path.push(hash);
                    Rc::new(File::open(path).unwrap())
                })
                .clone()
        });
        file
    }
}
