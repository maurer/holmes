//! This module defines the interface which a fact database must present to
//! be used as a backend by the Holmes engine.
use std::error::Error;
use pg::dyn::{Value, Type};
use engine::types::{Fact, Clause, Predicate};
/// This is a universal result type, allowing any `Error`
pub type Result<T> = ::std::result::Result<T, Box<Error>>;

/// This is the interface which a fact database must satisfy to be used by the
/// Holmes engine.
pub trait FactDB {
  /// Adds a new fact to the database, returning false if the fact was already
  /// present in the database, and true if it was inserted.
  fn insert_fact(&mut self, fact : &Fact) -> Result<bool>;

  /// Registers a new type with the database.
  /// This is unstable, and will likely need to be moved to the initialization
  /// of the database object in order to allow reconnecting to an existing
  /// database.
  fn add_type(&mut self, type_ : Type) -> Result<()>;

  /// Looks for a named type in the database's registry.
  /// This function is primarily useful for the DSL shorthand for constructing
  /// queries, since it allows you to use names of types when declaring
  /// functions rather than type objects.
  fn get_type(&self, type_str : &str) -> Option<Type>;

  /// Fetches a predicate by name
  fn get_predicate(&self, pred_name : &str) -> Option<&Predicate>;

  /// Persists a predicate by name
  fn new_predicate(&mut self, pred : &Predicate) -> Result<()>;

  /// Attempt to match the right hand side of a datalog rule against the
  /// database, returning a list of solution assignments to the bound
  /// variables.
  fn search_facts(&self, query : &Vec<Clause>)
    -> Result<Vec<Vec<Value>>>;
}
