//! This module defines the interface which a fact database must present to
//! be used as a backend by the Holmes engine.
use pg::dyn::{Value, Type};
/// Abstract reference to a cache
pub type CacheId = i64;
/// Abstract reference to a particular fact in the database
pub type FactId = i32;
use engine::types::{Fact, Clause, Predicate};

use std::result::Result;

/// This is the interface which a fact database must satisfy to be used by the
/// Holmes engine.
pub trait FactDB {
    /// FactDB implementation provided error type
    type Error: ::std::error::Error;
    /// Adds a new fact to the database, returning false if the fact was already
    /// present in the database, and true if it was inserted.
    fn insert_fact(&self, fact: &Fact) -> Result<bool, Self::Error>;

    /// Registers a new type with the database.
    /// This is unstable, and will likely need to be moved to the initialization
    /// of the database object in order to allow reconnecting to an existing
    /// database.
    fn add_type(&self, type_: Type) -> Result<(), Self::Error>;

    /// Looks for a named type in the database's registry.
    /// This function is primarily useful for the DSL shorthand for constructing
    /// queries, since it allows you to use names of types when declaring
    /// functions rather than type objects.
    fn get_type(&self, type_str: &str) -> Option<Type>;

    /// Fetches a predicate by name
    fn get_predicate(&self, pred_name: &str) -> Option<Predicate>;

    /// Persists a predicate by name
    fn new_predicate(&self, pred: &Predicate) -> Result<(), Self::Error>;

    /// Creates a cache table for a new rule, returning a handle
    fn new_rule_cache(&self, pred: Vec<String>) -> Result<CacheId, Self::Error>;

    /// Update
    fn cache_hit(&self, cache: CacheId, Vec<FactId>) -> Result<(), Self::Error>;

    /// Attempt to match the right hand side of a datalog rule against the
    /// database, returning a list of solution assignments to the bound
    /// variables.
    /// Optionally provide a cache handle to have the db filter already
    /// processed results based on a provided cache.
    fn search_facts(&self,
                    query: &Vec<Clause>,
                    cache: Option<CacheId>)
                    -> Result<Vec<(Vec<FactId>, Vec<Value>)>, Self::Error>;
}
