//! Holmes/Datalog Execution Engine
//!
//! This module contains the logic for rule execution and non-persistent state
//! maintenance.

pub mod types;

use std::collections::hash_map::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use pg::dyn::{Value, Type};
use pg::dyn::values;
use self::types::{Fact, Rule, Func, Predicate, Clause, Expr, BindExpr};
use fact_db::{FactDB, CacheId, FactId};

/// The `Engine` type contains the context necessary to run a Holmes program
pub struct Engine<FE: ::std::error::Error + Send + 'static, FDB: FactDB<Error = FE>> {
    fact_db: FDB,
    funcs: HashMap<String, Func>,
    rules: HashMap<String, Vec<Rule>>,
    rule_cache: HashMap<Rule, CacheId>,
}

#[allow(missing_docs)]
mod errors {
    error_chain! {
        errors {
            Invalid(msg: String) {
                description("Invalid Request")
                display("Invalid Request: {}", msg)
            }
            Internal(msg: String) {
                description("Internal Error (bug)")
                display("Internal Error (bug): {}", msg)
            }
            Type(msg: String) {
                description("Type Error")
                display("Type Error: {}", msg)
            }
            FactDB {
                description("FactDB propagated error")
            }
        }
    }
}

pub use self::errors::*;

fn substitute(clause: &Clause, ans: &Vec<Value>) -> Fact {
    use self::types::MatchExpr::*;
    Fact {
        pred_name: clause.pred_name.clone(),
        args: clause.args
            .iter()
            .map(|slot| {
                match slot {
                    &Unbound => panic!("Unbound is not allowed in substituted facts"),
                    &SubStr(_, _, _) => panic!("Substring is not allowed in substituted facts"),
                    &Var(ref n) => ans[*n as usize].clone(),
                    &Const(ref v) => v.clone(),
                }
            })
            .collect(),
    }
}

impl<FE, FDB> Engine<FE, FDB>
    where FE: ::std::error::Error + Send + 'static,
          FDB: FactDB<Error = FE>
{
    /// Create a fresh engine by handing it a fact database to use
    pub fn new(db: FDB) -> Self {
        Engine {
            fact_db: db,
            funcs: HashMap::new(),
            rules: HashMap::new(),
            rule_cache: HashMap::new(),
        }
    }
    /// Seach the type registry for a named type
    /// If present, it returns `Some(type)`, otherwise `None`
    pub fn get_type(&self, name: &str) -> Option<Type> {
        self.fact_db.get_type(name)
    }
    /// Register a new type
    /// This type must be a named type (e.g. type.name() should return `Some`)
    pub fn add_type(&mut self, type_: Type) -> Result<()> {
        Ok(try!(self.fact_db.add_type(type_).chain_err(|| ErrorKind::FactDB)))
    }
    /// Register a new predicate
    /// This defines the type signature of a predicate and persists it
    ///
    /// * Predicates must have at least one argument
    /// * Predicates must have a unique name
    /// * While using the `pg` backend, their name must be lowercase ascii or '_'
    pub fn new_predicate(&mut self, pred: &Predicate) -> Result<()> {

        // Verify we have at least one argument
        if pred.types.len() == 0 {
            bail!(ErrorKind::Invalid("Predicates must have at least one argument.".to_string()));
        }

        // Check for existing predicates/type issues
        match self.fact_db.get_predicate(&pred.name) {
            Some(p) => {
                if pred.types == p.types {
                    ()
                } else {
                    bail!(ErrorKind::Type(format!("{:?} != {:?}", pred.types, p.types)));
                }
            }
            None => (),
        }

        Ok(try!(self.fact_db.new_predicate(pred).chain_err(|| ErrorKind::FactDB)))
    }

    /// Adds a new fact to the database
    /// If the fact is already present, a new copy will not be added.
    ///
    /// * The relevant predicate must already be registered
    /// * The fact must be correctly typed
    pub fn new_fact(&mut self, fact: &Fact) -> Result<()> {
        match self.fact_db.get_predicate(&fact.pred_name) {
            Some(ref pred) => {
                if (fact.args.len() != pred.types.len()) ||
                   (!fact.args
                    .iter()
                    .zip(pred.types.iter())
                    .all(|(val, ty)| val.type_() == ty.clone())) {
                    bail!(ErrorKind::Type(format!("Fact ({:?}) does not \
                                                   match predicate ({:?})",
                                                  fact,
                                                  pred.types)));
                }
            }
            None => bail!(ErrorKind::Invalid("Predicate not registered".to_string())),
        }
        {
            if try!(self.fact_db.insert_fact(&fact).chain_err(|| ErrorKind::FactDB)) {
                for rule in self.rules
                    .get(&fact.pred_name)
                    .unwrap_or(&vec![])
                    .clone() {
                    self.run_rule(&rule);
                }
            }
            Ok(())
        }
    }

    /// Returns success in the appropriate type. This helper function is to
    /// support the EDSL, and it is not anticipated to be useful normally.
    pub fn nop(&mut self) -> Result<()> {
        Ok(())
    }

    // In an assignment statement, once the rhs has been computed, binds the
    // rhs value onto the expression on the left, using the state to check that
    // already bound variables are bound to the same things
    // It returns list of output states, each of which is a list of var bindings
    fn bind(&self, lhs: &BindExpr, rhs: Value, state: &Vec<Value>) -> Vec<Vec<Value>> {
        use self::types::BindExpr::*;
        use self::types::MatchExpr::*;
        match *lhs {
            // If we are unbound, we no-op
            Normal(Unbound) => vec![state.clone()],
            // Substring bindings don't make sense here
            Normal(SubStr(_, _, _)) => panic!("Substring binding in where clause not allowed"),
            // To bind to a variable,
            Normal(Var(v)) => {
                // If the variable is defined, check equality
                if v < state.len() {
                    if state[v] == rhs {
                        vec![state.clone()]
                    } else {
                        vec![]
                    }
                    // If the variable is to be defined, define it
                } else if v == state.len() {
                    let mut next = state.clone();
                    next.push(rhs.clone());
                    vec![next]
                    // Otherwise it is a malformed binding
                } else {
                    panic!("Variable out of range")
                }
            }
            Normal(Const(ref v)) => {
                if *v == rhs {
                    vec![state.clone()]
                } else {
                    vec![]
                }
            }
            Destructure(ref lhss) => {
                let rhss = match rhs.get().downcast_ref::<Vec<Value>>() {
                    Some(ref rhss) => rhss.iter(),
                    _ => panic!("Attempted to destructure non-list"),
                };
                let mut next = vec![state.clone()];
                for (lhs, rhs) in lhss.iter().zip(rhss) {
                    let mut next_next = vec![];
                    for state in next {
                        next_next.extend(self.bind(lhs, rhs.clone(), &state));
                    }
                    next = next_next;
                }
                next
            }
            Iterate(ref inner) => {
                let rhss = match rhs.get().downcast_ref::<Vec<Value>>() {
                    Some(ref rhss) => rhss.iter(),
                    _ => panic!("Attempted to destructure non-list"),
                };
                rhss.flat_map(|rhs| self.bind(inner, rhs.clone(), &state))
                    .collect()
            }
        }
    }

    // Evaluates an expression, given a set of bindings to variables
    fn eval(&self, expr: &Expr, subs: &Vec<Value>) -> Value {
        use self::types::Expr::*;
        match *expr {
            Var(var) => subs[var as usize].clone(),
            Val(ref val) => val.clone(),
            App(ref fun_name, ref args) => {
                let arg_vals: Vec<Value> = args.iter()
                    .map(|arg_expr| self.eval(arg_expr, subs))
                    .collect();
                let arg = if arg_vals.len() == 1 {
                    arg_vals[0].clone()
                } else {
                    values::Tuple::new(arg_vals) as Value
                };
                (self.funcs[fun_name].run)(arg)
            }
        }
    }

    fn rule_cache(&mut self, rule: &Rule) -> CacheId {
        match self.rule_cache.entry(rule.clone()) {
            Occupied(e) => *e.get(),
            Vacant(e) => {
                let cid = self.fact_db
                    .new_rule_cache(rule.body
                        .iter()
                        .map(|clause| clause.pred_name.clone())
                        .collect())
                    .unwrap();
                e.insert(cid);
                cid
            }
        }
    }

    // Run a rule once on all body clause matches we have not yet run it on
    // This function cascades via `new_rule`
    // TODO change recursive cascade to iterative cascade
    fn run_rule(&mut self, rule: &Rule) {
        let cache = self.rule_cache(&rule);
        let mut states: Vec<(Vec<FactId>, Vec<Value>)> =
            self.fact_db.search_facts(&rule.body, Some(cache)).unwrap();

        for where_clause in rule.wheres.iter() {
            let mut next_states: Vec<(Vec<FactId>, Vec<Value>)> = Vec::new();
            for state in states {
                let resp = self.eval(&where_clause.rhs, &state.1);
                next_states.extend(self.bind(&where_clause.lhs, resp, &state.1)
                    .into_iter()
                    .map(|x| (state.0.clone(), x)));
            }
            states = next_states;
        }
        for state in states {
            // TODO once we go multithreaded again, this could race, so I need to restructure this
            self.fact_db.cache_hit(cache, state.0).unwrap();
            self.new_fact(&substitute(&rule.head, &state.1)).unwrap();
        }
    }

    /// Given a query (similar to the rhs of a rule in Datalog), provide the set
    /// of satisfying answers in the database.
    pub fn derive(&self, query: &Vec<Clause>) -> Result<Vec<Vec<Value>>> {
        Ok(try!(self.fact_db.search_facts(query, None).chain_err(|| ErrorKind::FactDB))
            .into_iter()
            .map(|x| x.1)
            .collect())
    }

    /// Register a new rule with the database
    pub fn new_rule(&mut self, rule: &Rule) -> Result<()> {
        for pred in &rule.body {
            match self.rules.entry(pred.pred_name.clone()) {
                Vacant(entry) => {
                    entry.insert(vec![rule.clone()]);
                }
                Occupied(mut entry) => entry.get_mut().push(rule.clone()),
            }
        }
        self.run_rule(rule);
        Ok(())
    }

    /// Register a new function with the database, to be called from within a
    /// rule
    ///
    /// Do not attempt to register a function name multiple times.
    // TODO: stop function reregistration, document restriction
    pub fn reg_func(&mut self, name: String, func: Func) -> Result<()> {
        self.funcs.insert(name, func);
        Ok(())
    }
}
