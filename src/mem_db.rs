//! This is a memory mock for the fact database interface.
//!
//! The primary purpose of this module is to allow for quick doctest-time
//! checks of the language. Any serious program should use a different backend,
//! as should the correctness tests.
//!
//! It is not built efficiently, and I do not intend to make it efficient - I'd
//! essentially be reimplementing many parts of a traditional database
//! (indexing, joins, etc).
use fact_db::{FactDB, FactId, CacheId};
use pg::dyn::{Value, Type};
use pg::dyn::types::default_types;
use engine::types::{Fact, Clause, Predicate, MatchExpr};
use std::collections::{HashMap, HashSet};

#[allow(missing_docs)]
mod errors {
    error_chain! {
        errors {
            Type(msg: String)
            Arg(msg: String)
        }
    }
}

pub use self::errors::*;

/// MemDB is an in-memory mock up of the fact database interface.
///
/// While it can be useful for quick tests, it should not be depended on for
/// anything serious, even if you want a standalone app. It is very slow and
/// persists nothing.
pub struct MemDB {
    facts: HashMap<FactId, Fact>,
    facts_set: HashSet<Fact>,
    next_id: FactId,
    rule_cache: Vec<HashSet<Vec<FactId>>>,
    types: HashMap<String, Type>,
    preds: HashMap<String, Predicate>,
}

impl MemDB {
    /// Creates a fresh empty `MemDB`.
    pub fn new() -> MemDB {
        MemDB {
            facts: HashMap::new(),
            next_id: 0,
            facts_set: HashSet::new(),
            rule_cache: Vec::new(),
            types: default_types()
                .iter()
                .filter_map(|type_| type_.name().map(|name| (name.to_owned(), type_.clone())))
                .collect(),
            preds: HashMap::new(),
        }
    }
}

fn raw_option<T>(some: bool, val: T) -> Option<T> {
    if some { Some(val) } else { None }
}

impl FactDB for MemDB {
    type Error = Error;
    fn new_rule_cache(&mut self, _preds: Vec<String>) -> Result<CacheId> {
        self.rule_cache.push(HashSet::new());
        Ok((self.rule_cache.len() - 1) as CacheId)
    }
    fn cache_hit(&mut self, cache: CacheId, facts: Vec<FactId>) -> Result<()> {
        self.rule_cache[cache as usize].insert(facts);
        Ok(())
    }
    fn insert_fact(&mut self, fact: &Fact) -> Result<bool> {
        if self.facts_set.contains(fact) {
            return Ok(false);
        };
        let id = self.next_id;
        self.next_id += 1;
        self.facts.insert(id, fact.clone());
        self.facts_set.insert(fact.clone());
        Ok(true)
    }
    fn add_type(&mut self, type_: Type) -> Result<()> {
        let name = type_.name().ok_or(ErrorKind::Arg("Provided type had no name".to_string()))?;
        self.types.insert(name.to_string(), type_);
        Ok(())
    }
    fn get_type(&self, type_str: &str) -> Option<Type> {
        self.types.get(type_str).map(|x| x.clone())
    }
    fn get_predicate(&self, pred_name: &str) -> Option<&Predicate> {
        self.preds.get(pred_name)
    }
    fn new_predicate(&mut self, pred: &Predicate) -> Result<()> {
        match self.preds.get(&pred.name) {
            Some(exist) => {
                if exist == pred {
                    return Ok(());
                } else {
                    bail!(ErrorKind::Type(format!("Predicate already registered with different \
                                                   type.\nExisting: {:?}\nNew: {:?}",
                                                  exist,
                                                  pred)));
                }
            }
            None => (),
        }
        self.preds.insert(pred.name.to_string(), pred.clone());
        Ok(())
    }
    fn search_facts(&self,
                    query: &Vec<Clause>,
                    cache: Option<CacheId>)
                    -> Result<Vec<(Vec<FactId>, Vec<Value>)>> {
        Ok(query.iter().fold(vec![(vec![], vec![])], |asgns, clause| {
            asgns.iter()
                .flat_map(|asgn| {
                    self.facts.iter().flat_map(move |(id, fact)| {
                        (if fact.pred_name == clause.pred_name {
                                fact.args
                                    .iter()
                                    .zip(clause.args.iter())
                                    .fold(Some({
                                              let mut nasgn = asgn.clone();
                                              nasgn.0.push(*id);
                                              nasgn
                                          }),
                                          |o_asgn, (val, arg)| {
                                        o_asgn.and_then(|asgn| {
                                            match *arg {
                                                MatchExpr::Unbound => Some(asgn),
                                                MatchExpr::Var(var) => {
                                                    if var >= asgn.1.len() {
                                                        let mut next = asgn.clone();
                                                        next.1.push(val.clone());
                                                        Some(next)
                                                    } else {
                                                        raw_option(&asgn.1[var] == val, asgn)
                                                    }
                                                }
                                                MatchExpr::SubStr(_, _, _) => {
                                                    panic!("Substring not implemented in memdb")
                                                }
                                                MatchExpr::Const(ref k) => {
                                                    raw_option(k == val, asgn)
                                                }
                                            }
                                        })
                                    })
                            } else {
                                None
                            })
                            .into_iter()
                    })
                })
                .filter(|&(ref facts, _)| {
                    match cache {
                        Some(c) => !self.rule_cache[c as usize].contains(facts),
                        None => true,
                    }
                })
                .collect()
        }))
    }
}
