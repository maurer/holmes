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
use engine::types::{Fact, Clause, Predicate, MatchExpr, Var, Projection};
use std::collections::{HashMap, HashSet};

use std::cell::{RefCell, Cell};

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

#[derive(Hash, PartialEq, Eq, Clone)]
struct Assignment {
    inner: Vec<Option<Value>>,
}

impl ::std::ops::Index<Var> for Assignment {
    type Output = Value;
    fn index(&self, index: Var) -> &Value {
        self.get(index)
    }
}

impl Assignment {
    fn new() -> Self {
        Assignment { inner: vec![] }
    }
    fn undefined(&self, v: Var) -> bool {
        if self.inner.len() <= v {
            true
        } else {
            self.inner[v as usize].is_none()
        }
    }
    fn set(&mut self, var: Var, val: Value) {
        if self.inner.len() <= var {
            self.inner.resize(var + 1 as usize, None);
        }
        self.inner[var as usize] = Some(val)
    }
    fn get(&self, var: Var) -> &Value {
        self.inner[var as usize].as_ref().unwrap()
    }
    fn complete(self) -> Vec<Value> {
        self.inner.into_iter().map(|x| x.unwrap()).collect()
    }
    fn mask(&self, vars: &[Var]) -> Assignment {
        let mut asgn = Assignment {
            inner: self.inner
                .iter()
                .enumerate()
                .map(|(n, ov)| -> Option<Value> {
                    match *ov {
                        Some(ref v) => raw_option(vars.contains(&n), v.clone()),
                        None => None,
                    }
                })
                .collect(),
        };
        asgn.normalize();
        asgn
    }
    fn normalize(&mut self) {
        let mut max_dex: isize = -1;
        for (i, v) in self.inner.iter().enumerate() {
            if v.is_some() {
                max_dex = i as isize;
            }
        }
        self.inner.truncate((max_dex + 1) as usize);
    }
    fn by_example(&self, example: &Self) -> Self {
        Assignment {
            inner: self.inner
                .iter()
                .zip(example.inner.iter())
                .map(|(base, ex)| { if ex.is_some() { base.clone() } else { None } })
                .collect(),
        }
    }
    fn extend(&mut self, extension: &Self) {
        if self.inner.len() < extension.inner.len() {
            self.inner.resize(extension.inner.len(), None)
        }
        for (i, v) in extension.inner.iter().enumerate() {
            if v.is_some() {
                self.inner[i] = v.clone();
            }
        }
    }
}

struct Index {
    query: Vec<Clause>,
    // If this query is of arity >1, this contains the real index structure
    // The first index is where in the rule the hole is, e.g. for
    // p(x, y), q(y, z), r(z, a)
    // the first element would be a mapping from legal assignments to [y], into legal
    // assignments to [z, a]
    // The second element would be a mapping from legal assignemnts to [y, z], into legal
    // assignments to [x, a]
    // The third element would be a mapping from legal assignments to z, into legal assignments
    // to [x, y]
    holy: Vec<(CacheId, HashMap<Assignment, Vec<(Vec<FactId>, Assignment)>>)>,
}

impl Index {
    fn new(icid: CacheId, clauses: Vec<Clause>, db: &MemDB) -> Result<Index> {
        if clauses.len() == 0 {
            panic!("Tried to create an index for the empty query.")
        }
        // Create subcaches, needed to efficiently regenerat our own cache when new facts are added
        let holy: Result<Vec<(CacheId, HashMap<Assignment, Vec<(Vec<FactId>, Assignment)>>)>> =
            if clauses.len() == 1 {
                // Don't try to cascade and generate a subindex for the null query
                Ok(vec![])
            } else {
                (0..clauses.len())
                    .map(|skip_id: usize| {
                        let subquery: Vec<Clause> = clauses.iter()
                            .enumerate()
                            .filter_map(|(i, c)| raw_option(i != skip_id, c.clone()))
                            .collect();
                        let cid: CacheId = db.new_rule_cache(&subquery)?;
                        let mut asgns = HashMap::new();
                        for (fact_ids, asgn) in db.fetch_cache_update(cid).into_iter() {
                            let uni = asgn.mask(&clauses[skip_id].free());
                            asgns.entry(uni).or_insert(vec![]).push((fact_ids, asgn));
                        }
                        Ok((cid, asgns))
                    })
                    .collect()
            };
        let out = db.search_asgns(&clauses, None)?;
        db.publish_cache_update(icid, &out);
        Ok(Index {
            query: clauses,
            holy: holy?,
        })
    }
    fn update(&mut self, icid: CacheId, fid: FactId, fact: &Fact, db: &MemDB) {
        if self.query.len() == 1 {
            // We have no subcaches or matching to do, just substitute out
            extract(&self.query[0], fact)
                .map(|asgn| db.publish_cache_update(icid, &[(vec![fid], asgn)]));
        } else {
            // For each clause we could potentially be matching,
            for (clause_index, clause) in
                self.query.iter().enumerate().filter(|&(_, c)| c.pred_name == fact.pred_name) {
                // Figure out what the current clause would match as
                let asgn = match extract(&clause, fact) {
                    Some(x) => x,
                    None => continue, // We don't match, go to the next clause
                };
                // Update the index with any incoming entries
                let sub_id = self.holy[clause_index].0;
                let holy = &mut self.holy[clause_index].1;
                let free = clause.free();
                for (fact_ids, asgn) in db.fetch_cache_update(sub_id) {
                    let uni = asgn.mask(&free);
                    holy.entry(uni).or_insert(vec![]).push((fact_ids, asgn));
                }
                // Fetch unifying answers
                let example = match holy.keys().next() {
                    Some(x) => x,
                    None => continue,
                };
                let key = asgn.by_example(example);
                match holy.get(&key) {
                    Some(subs) => {
                        let out: Vec<_> = subs.iter()
                            .map(|&(ref fact_ids, ref base)| {
                                let mut fact_ids = fact_ids.clone();
                                fact_ids.insert(clause_index, fid);
                                let mut base = base.clone();
                                base.extend(&asgn);
                                (fact_ids, base)
                            })
                            .collect();
                        db.publish_cache_update(icid, &out);
                    }
                    None => continue,
                }
            }
        }
    }
}

// TODO this function actually can't be written with unrestricted projection,
// since the projection can use information from elsewhere in the match
// If I want to use this for real work, I'll need to figure out how to return
// maybe matches" and then actually run the projection and verify the fuzzy
// matches right before outputting
fn eval(proj: &Projection, fact: &Fact) -> Value {
    match *proj {
        Projection::Slot(n) => fact.args[n].clone(),
        _ => panic!("See todo, eval broken"),
    }
}

fn extract(clause: &Clause, fact: &Fact) -> Option<Assignment> {
    if clause.pred_name != fact.pred_name {
        return None;
    }
    let mut asgn = Assignment::new();
    for &(ref proj, ref mat) in clause.args.iter() {
        match *mat {
            MatchExpr::Unbound => (),
            MatchExpr::Const(ref val) => {
                if val != &eval(proj, fact) {
                    return None;
                }
            }
            MatchExpr::Var(ref v) => asgn.set(*v, eval(proj, fact)),
        }
    }
    Some(asgn)
}

pub enum GcPolicy {
    Never,
    Size(usize),
}

/// MemDB is an in-memory mock up of the fact database interface.
///
/// While it can be useful for quick tests, it should not be depended on for
/// anything serious, even if you want a standalone app. It is very slow and
/// persists nothing.
///
/// MemDB does not currently support projections in any meaninful way. It will
/// treat all projections as though they are simply the next column in sequence
/// for the predicate they are attached to.
pub struct MemDB {
    facts: RefCell<HashMap<FactId, Fact>>,
    facts_set: RefCell<HashSet<Fact>>,
    next_id: Cell<FactId>,
    next_cache: Cell<CacheId>,
    // Map from predicate, onto arities of caches which need updating for that predicate
    pred_cache: RefCell<HashMap<String, Vec<Vec<CacheId>>>>,
    // Map from cache IDs onto not-yet-used outputs
    cache_out: RefCell<HashMap<CacheId, Vec<(Vec<FactId>, Assignment)>>>,
    cache_index: RefCell<HashMap<CacheId, Index>>,
    types: RefCell<HashMap<String, Type>>,
    preds: RefCell<HashMap<String, Predicate>>,
    gc_policy: GcPolicy,
}

impl MemDB {
    pub fn new() -> MemDB {
        MemDB::new_full(GcPolicy::Never)
    }
    /// Creates a fresh empty `MemDB`.
    pub fn new_full(gc_policy: GcPolicy) -> MemDB {
        MemDB {
            facts: RefCell::new(HashMap::new()),
            next_id: Cell::new(0),
            next_cache: Cell::new(0),
            facts_set: RefCell::new(HashSet::new()),
            pred_cache: RefCell::new(HashMap::new()),
            cache_out: RefCell::new(HashMap::new()),
            cache_index: RefCell::new(HashMap::new()),
            types: RefCell::new(default_types()
                .iter()
                .filter_map(|type_| type_.name().map(|name| (name.to_owned(), type_.clone())))
                .collect()),
            preds: RefCell::new(HashMap::new()),
            gc_policy: gc_policy,
        }
    }
    fn gc_old(&self) {
        let num_purge = self.facts.borrow().len() / 2;
        let purge_ids = self.facts.borrow().keys().take(num_purge).cloned().collect::<Vec<_>>();
        for fact_id in purge_ids.iter() {
            let fact = (self.facts.borrow_mut().remove(fact_id)).unwrap();
            self.facts_set.borrow_mut().remove(&fact);
        }
        for (_, cache) in self.cache_out.borrow_mut().iter_mut() {
            cache.retain(|&(ref fids, _)| {
                fids.iter().all(|fid| self.facts.borrow().contains_key(fid))
            });
        }
    }
    fn fetch_cache_update(&self, c: CacheId) -> Vec<(Vec<FactId>, Assignment)> {
        self.cache_out.borrow_mut().remove(&c).unwrap_or(vec![])
    }
    fn publish_cache_update(&self, c: CacheId, asgns: &[(Vec<FactId>, Assignment)]) {
        self.cache_out.borrow_mut().entry(c).or_insert(vec![]).extend_from_slice(asgns);
    }
    fn search_asgns(&self,
                    query: &Vec<Clause>,
                    cache: Option<CacheId>)
                    -> Result<Vec<(Vec<FactId>, Assignment)>> {
        match cache {
            Some(cid) => return Ok(self.fetch_cache_update(cid)),
            None => (),
        };

        Ok(query.iter().fold(vec![(vec![], Assignment::new())],
                             |asgns: Vec<(Vec<FactId>, Assignment)>, clause| {
            let facts = self.facts.borrow();
            asgns.iter()
                .flat_map(|asgn| {
                    facts.iter().flat_map(move |(id, fact)| {
                        (if fact.pred_name == clause.pred_name {
                                fact.args
                                    .iter()
                                    .zip(clause.args.iter())
                                    .fold(Some({
                                              let mut nasgn = asgn.clone();
                                              nasgn.0.push(*id);
                                              nasgn
                                          }),
                                          |o_asgn, (val, &(ref _proj, ref arg))| {
                                        o_asgn.and_then(|asgn| {
                                            match *arg {
                                                MatchExpr::Unbound => Some(asgn),
                                                MatchExpr::Var(var) => {
                                                    if asgn.1.undefined(var) {
                                                        let mut next = asgn.clone();
                                                        next.1.set(var, val.clone());
                                                        Some(next)
                                                    } else {
                                                        raw_option(&asgn.1[var] == val, asgn)
                                                    }
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
                .collect()
        }))
    }
}

fn raw_option<T>(some: bool, val: T) -> Option<T> {
    if some { Some(val) } else { None }
}

impl FactDB for MemDB {
    type Error = Error;
    fn new_rule_cache(&self, clauses: &Vec<Clause>) -> Result<CacheId> {
        let id = self.next_cache.get();
        self.next_cache.set(id + 1);
        let index = Index::new(id, clauses.clone(), self)?;
        self.cache_index.borrow_mut().insert(id, index);
        let mut preds: Vec<_> = clauses.iter().map(|x| x.pred_name.clone()).collect();
        preds.sort();
        preds.dedup();
        let arity = clauses.len();
        let mut pc = self.pred_cache.borrow_mut();
        for pred in preds {
            let mut entry = pc.entry(pred).or_insert(vec![]);
            if entry.len() < arity {
                entry.resize(arity, vec![]);
            }
            entry[arity - 1].push(id);
        }
        Ok(id)
    }
    fn insert_fact(&self, fact: &Fact) -> Result<bool> {
        if self.facts_set.borrow().contains(fact) {
            return Ok(false);
        };
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.facts.borrow_mut().insert(id, fact.clone());
        self.facts_set.borrow_mut().insert(fact.clone());
        for arity in self.pred_cache.borrow().get(&fact.pred_name).unwrap_or(&vec![]).clone() {
            for cid in arity {
                let mut index: Index = self.cache_index.borrow_mut().remove(&cid).unwrap();
                index.update(cid, id, fact, self);
                self.cache_index.borrow_mut().insert(cid, index);
            }
        }
        match self.gc_policy {
            GcPolicy::Size(n) if n < self.facts_set.borrow().len() => self.gc_old(),
            _ => (),
        }
        Ok(true)
    }
    fn add_type(&self, type_: Type) -> Result<()> {
        let name = type_.name().ok_or(ErrorKind::Arg("Provided type had no name".to_string()))?;
        self.types.borrow_mut().insert(name.to_string(), type_);
        Ok(())
    }
    fn get_type(&self, type_str: &str) -> Option<Type> {
        self.types.borrow().get(type_str).cloned()
    }
    fn get_predicate(&self, pred_name: &str) -> Option<Predicate> {
        self.preds.borrow().get(pred_name).cloned()
    }
    fn new_predicate(&self, pred: &Predicate) -> Result<()> {
        match self.preds.borrow().get(&pred.name) {
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
        self.preds.borrow_mut().insert(pred.name.to_string(), pred.clone());
        Ok(())
    }
    fn search_facts(&self,
                    query: &Vec<Clause>,
                    cache: Option<CacheId>)
                    -> Result<Vec<(Vec<FactId>, Vec<Value>)>> {
        Ok(self.search_asgns(query, cache)?
            .into_iter()
            .map(|(ids, asgn)| (ids, asgn.complete()))
            .collect())
    }
}
