//! This is a memory mock for the fact database interface.
//!
//! The primary purpose of this module is to allow for quick doctest-time
//! checks of the language. Any serious program should use a different backend,
//! as should the correctness tests.
//!
//! It is not built efficiently, and I do not intend to make it efficient - I'd
//! essentially be reimplementing many parts of a traditional database
//! (indexing, joins, etc).
use fact_db::{FactDB, Result};
use pg::dyn::{Value, Type};
use engine::types::{Fact, Clause, Predicate, MatchExpr};
use std::collections::{HashMap, HashSet};

/// MemDB is an in-memory mock up of the fact database interface.
///
/// While it can be useful for quick tests, it should not be depended on for
/// anything serious, even if you want a standalone app. It is very slow and
/// persists nothing.
pub struct MemDB {
  facts : HashSet<Fact>,
  types : HashMap<String, Type>,
  preds : HashMap<String, Predicate>
}

impl MemDB {
  /// Creates a fresh empty `MemDB`.
  pub fn new() -> MemDB {
    MemDB {
      facts : HashSet::new(),
      types : HashMap::new(),
      preds : HashMap::new()
    }
  }
}

fn raw_option<T>(some : bool, val : T) -> Option<T> {
  if some {
    Some(val)
  } else {
    None
  }
}

impl FactDB for MemDB {
  fn insert_fact(&mut self, fact : &Fact) -> Result<bool> {
    if self.facts.contains(fact) {
      return Ok(false)
    };
    self.facts.insert(fact.clone());
    Ok(true)
  }
  fn add_type(&mut self, type_ : Type) -> Result<()> {
    self.types.insert(type_.name().unwrap().to_string(), type_);
    Ok(())
  }
  fn get_type(&self, type_str : &str) -> Option<Type> {
    self.types.get(type_str).map(|x|{x.clone()})
  }
  fn get_predicate(&self, pred_name : &str) -> Option<&Predicate> {
    self.preds.get(pred_name)
  }
  fn new_predicate(&mut self, pred : &Predicate) -> Result<()> {
    self.preds.insert(pred.name.to_string(), pred.clone());
    Ok(())
  }
  fn search_facts(&self, query : &Vec<Clause>) -> Result<Vec<Vec<Value>>> {
    Ok(query.iter().fold(vec![vec![]], |asgns, clause| {
      asgns.iter().flat_map(|asgn| {
        self.facts.iter().flat_map(move |fact| {
          (if fact.pred_name == clause.pred_name {
            fact.args.iter().zip(clause.args.iter())
                .fold(Some(asgn.clone()), |o_asgn, (val, arg)| {
                  o_asgn.and_then(|asgn| {
                    match *arg {
                      MatchExpr::Unbound => Some(asgn),
                      MatchExpr::Var(var) =>
                        if var >= asgn.len() {
                          let mut next = asgn.clone();
                          next.push(val.clone());
                          Some(next)
                        } else {
                          raw_option(&asgn[var] == val, asgn)
                        },
                      MatchExpr::Const(ref k) =>
                        raw_option(k == val, asgn)
                    }
                  })
                })
          } else {
            None
          }).into_iter()
        })
      }).collect()
    }))
  }
}
