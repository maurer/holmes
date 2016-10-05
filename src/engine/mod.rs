//! Holmes/Datalog Execution Engine
//!
//! This module contains the logic for rule execution and non-persistent state
//! maintenance.

pub mod types;

use std::collections::hash_map::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use pg::dyn::{Value, Type};
use pg::dyn::values;
use self::types::{Fact, Rule, Func, Predicate, Clause, Expr, BindExpr};
use fact_db::FactDB;

/// The `Engine` type contains the context necessary to run a Holmes program
pub struct Engine {
  fact_db    : Box<FactDB>,
  funcs      : HashMap<String, Func>,
  rules      : HashMap<String, Vec<Rule>>,
  exec_cache : HashMap<Rule, HashSet<Vec<Value>>>
}

/// `engine::Error` describes ways that an attempt to input or run a Holmes
/// program could go wrong
#[derive(Debug)]
pub enum Error {
  /// An `Invalid` error means that bad input was given to the engine
  /// (e.g. the fault is with the caller)
  Invalid(String),
  /// An `Internal` error should not happen, and indicates a bug within the
  /// engine
  Internal(String),
  /// A `Type` error indicates a typing error in the Holmes program being
  /// operated on
  Type(String),
  /// A `Db` error indicates an error that the underlying fact database
  /// component has sent up to the engine
  Db(Box<::std::error::Error>)
}

impl ::std::convert::From<Box<::std::error::Error>> for Error {
  fn from(dbe : Box<::std::error::Error>) -> Self {
    Error::Db(dbe)
  }
}

impl ::std::fmt::Display for Error {
  fn fmt(&self, fmt : &mut ::std::fmt::Formatter)
        -> ::std::result::Result<(), ::std::fmt::Error> {
    match *self {
      Error::Invalid(ref s) => fmt.write_fmt(format_args!("Invalid request: {}", s)),
      Error::Internal(ref s) => fmt.write_fmt(format_args!("Internal problem (bug): {}", s)),
      Error::Type(ref s) => fmt.write_fmt(format_args!("Type error: {}", s)),
      Error::Db(ref s) => fmt.write_fmt(format_args!("FactDB problem: {}", s))
    }
  }
}

impl ::std::error::Error for Error {
  fn description(&self) -> &str {
    match *self {
      Error::Invalid(_) => "Invalid request",
      Error::Internal(_) => "Internal error (bug)",
      Error::Type(_) => "Type mismatch",
      Error::Db(_) => "Error in interaction with FactDB"
    }
  }
  fn cause(&self) -> Option<&::std::error::Error> {
    match *self {
      Error::Db(ref dbe) => Some(&**dbe),
      Error::Invalid(_) | Error::Internal(_) | Error::Type(_) => None
    }
  }
}

fn substitute(clause : &Clause, ans : &Vec<Value>) -> Fact {
  use self::types::MatchExpr::*;
  Fact {
    pred_name : clause.pred_name.clone(),
    args : clause.args.iter().map(|slot| {
      match slot {
        &Unbound       => panic!("Unbound is not allowed in substituted facts"),
        &SubStr(_,_,_) => panic!("Substring is not allowed in substituted facts"),
        &Var(ref n)    => ans[*n as usize].clone(),
        &Const(ref v) => v.clone()
      }
    }).collect()
  }
}

impl Engine {
  /// Create a fresh engine by handing it a fact database to use
  pub fn new(db : Box<FactDB>) -> Engine {
    Engine {
      fact_db    : db,
      funcs      : HashMap::new(),
      rules      : HashMap::new(),
      exec_cache : HashMap::new(),
    }
  }
  /// Seach the type registry for a named type
  /// If present, it returns `Some(type)`, otherwise `None`
  pub fn get_type(&self, name : &str) -> Option<Type> {
    self.fact_db.get_type(name)
  }
  /// Register a new type
  /// This type must be a named type (e.g. type.name() should return `Some`)
  pub fn add_type(&mut self, type_ : Type) -> Result<(), Error> {
    Ok(try!(self.fact_db.add_type(type_)))
  }
  /// Register a new predicate
  /// This defines the type signature of a predicate and persists it
  ///
  /// * Predicates must have at least one argument
  /// * Predicates must have a unique name
  /// * While using the `pg` backend, their name must be lowercase ascii or '_'
  pub fn new_predicate(&mut self, pred : &Predicate) -> Result<(), Error> {

    // Verify we have at least one argument
    if pred.types.len() == 0 {
      return Err(Error::Invalid("Predicates must have at least one argument.".to_string()));
    }

    // Check for existing predicates/type issues
    match self.fact_db.get_predicate(&pred.name) {
      Some(p) => {
        if pred.types == p.types {
          ()
        } else {
          return Err(Error::Type(format!("{:?} != {:?}", pred.types, p.types)))
        }
      }
      None => ()
    }

    Ok(try!(self.fact_db.new_predicate(pred)))
  }

  /// Adds a new fact to the database
  /// If the fact is already present, a new copy will not be added.
  ///
  /// * The relevant predicate must already be registered
  /// * The fact must be correctly typed
  pub fn new_fact(&mut self, fact : &Fact) -> Result<(), Error> {
    match self.fact_db.get_predicate(&fact.pred_name) {
      Some(ref pred) => {
        if (fact.args.len() != pred.types.len())
           || (!fact.args.iter().zip(pred.types.iter()).all(|(val, ty)| {val.type_() == ty.clone()})) {
          return Err(Error::Type(format!("Fact ({:?}) does not match predicate ({:?})", fact, pred.types)))
        }
      }
      None => return Err(Error::Invalid("Predicate not registered".to_string()))
    }
    {
      if try!(self.fact_db.insert_fact(&fact)) {
        for rule in self.rules.get(&fact.pred_name)
                              .unwrap_or(&vec![]).clone() {
          self.run_rule(&rule);
        }
      }
      Ok(())
    }
  }

  // In an assignment statement, once the rhs has been computed, binds the
  // rhs value onto the expression on the left, using the state to check that
  // already bound variables are bound to the same things
  // It returns list of output states, each of which is a list of var bindings
  fn bind(&self, lhs : &BindExpr, rhs : Value, state : &Vec<Value>) -> Vec<Vec<Value>> {
    use self::types::BindExpr::*;
    use self::types::MatchExpr::*;
    match *lhs {
      // If we are unbound, we no-op
      Normal(Unbound) => vec![state.clone()],
      // Substring bindings don't make sense here
      Normal(SubStr(_,_,_)) => panic!("Substring binding in where clause not allowed"),
      // To bind to a variable,
      Normal(Var(v))  => {
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
          _ => panic!("Attempted to destructure non-list")
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
      },
      Iterate(ref inner) => {
        let rhss = match rhs.get().downcast_ref::<Vec<Value>>() {
          Some(ref rhss) => rhss.iter(),
          _ => panic!("Attempted to destructure non-list")
        };
        rhss.flat_map(|rhs| {
          self.bind(inner, rhs.clone(), &state)
        }).collect()
      }
    }
  }

  // Evaluates an expression, given a set of bindings to variables
  fn eval(&self, expr : &Expr, subs : &Vec<Value>) -> Value {
    use self::types::Expr::*;
    match *expr {
      Var(var) => subs[var as usize].clone(),
      Val(ref val) => val.clone(),
      App(ref fun_name, ref args) => {
        let arg_vals : Vec<Value> = args.iter().map(|arg_expr|{
          self.eval(arg_expr, subs)
        }).collect();
        let arg = if arg_vals.len() == 1 {
          arg_vals[0].clone()
        } else {
          values::Tuple::new(arg_vals) as Value
        };
        (self.funcs[fun_name].run)(arg)
      }
    }
  }

  // Checks whether we have already run this particular rule on this particular
  // assignment of variables. If we have, it returns false, and we can skip
  // the rerun. If we have not, it updates the cache so that we have, and
  // returns true.
  fn rule_cache_miss(&mut self, rule : &Rule, args : &Vec<Value>)
    -> bool {
    match self.exec_cache.entry(rule.clone()) {
      Vacant(entry) => {
        let mut cache = HashSet::new();
        cache.insert(args.clone());
        entry.insert(cache);
        true
      }
      Occupied(mut entry) => {
        if !entry.get().contains(args) {
          entry.get_mut().insert(args.clone());
          true
        } else {
          false
        }
      }
    }
  }

  // Run a rule once on all body clause matches we have not yet run it on
  // This function cascades via `new_rule`
  // TODO change recursive cascade to iterative cascade
  fn run_rule(&mut self, rule : &Rule) {
    let anss = self.fact_db.search_facts(&rule.body).unwrap();
    let mut states : Vec<Vec<Value>> =
        anss.iter()
            .filter(|ans| {self.rule_cache_miss(&rule, &ans)})
            .map(|ans| {ans.clone()})
            .collect();

    for where_clause in rule.wheres.iter() {
      let mut next_states : Vec<Vec<Value>> = Vec::new();
      for state in states {
        let resp = self.eval(&where_clause.rhs, &state);
        next_states.extend(
          self.bind(&where_clause.lhs, resp, &state));
      }
      states = next_states;
    }
    for state in states {
      self.new_fact(&substitute(&rule.head, &state)).unwrap();
    }
  }

  /// Given a query (similar to the rhs of a rule in Datalog), provide the set
  /// of satisfying answers in the database.
  pub fn derive(&self, query : &Vec<Clause>) -> Result<Vec<Vec<Value>>, Error> {
    Ok(try!(self.fact_db.search_facts(query)))
  }

  /// Register a new rule with the database
  pub fn new_rule(&mut self, rule : &Rule) -> Result<(), Error> {
    for pred in &rule.body {
      match self.rules.entry(pred.pred_name.clone()) {
        Vacant(entry) => {entry.insert(vec![rule.clone()]);}
        Occupied(mut entry) => entry.get_mut().push(rule.clone())
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
  pub fn reg_func(&mut self, name : String, func : Func) {
      self.funcs.insert(name, func);
  }
}
