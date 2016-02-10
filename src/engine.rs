use std::collections::hash_map::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use native_types::*;
use pg::dyn::{Value, Type};
use pg::dyn::values;
use pg::PgDB;
use pg;

pub struct Engine {
  fact_db    : PgDB,
  funcs      : HashMap<String, HFunc>,
  rules      : HashMap<String, Vec<Rule>>,
  exec_cache : HashMap<Rule, HashSet<Vec<Value>>>
}

#[derive(Debug)]
pub enum Error {
  Invalid(String),
  Internal(String),
  Type(String),
  Db(String)
}

impl ::std::convert::From<pg::Error> for Error {
  fn from(dbe : pg::Error) -> Self {
    Error::Db(format!("{:?}", dbe))
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
}

fn substitute(clause : &Clause, ans : &Vec<Value>) -> Fact {
  use native_types::MatchExpr::*;
  Fact {
    pred_name : clause.pred_name.clone(),
    args : clause.args.iter().map(|slot| {
      match slot {
        &Unbound       => panic!("Unbound is not allowed in substituted facts"),
        &Var(ref n)    => ans[*n as usize].clone(),
        &Const(ref v) => v.clone()
      }
    }).collect()
  }
}

impl Engine {
  pub fn new(db : PgDB) -> Engine {
    Engine {
      fact_db    : db,
      funcs      : HashMap::new(),
      rules      : HashMap::new(),
      exec_cache : HashMap::new(),
    }
  }
  pub fn get_type(&self, name : &str) -> Option<Type> {
    self.fact_db.get_type(name)
  }
  pub fn add_type(&mut self, type_ : Type) -> Result<(), Error> {
    Ok(try!(self.fact_db.add_type(type_)))
  }
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

  fn bind(&self, lhs : &BindExpr, rhs : Value, state : &Vec<Value>) -> Vec<Vec<Value>> {
    match *lhs {
      // If we are unbound, we no-op
      Normal(Unbound) => vec![state.clone()],
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

  fn eval(&self, expr : &Expr, subs : &Vec<Value>) -> Value {
    use native_types::Expr::*;
    match *expr {
      EVar(var) => subs[var as usize].clone(),
      EVal(ref val) => val.clone(),
      EApp(ref fun_name, ref args) => {
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
      assert!(self.new_fact(&substitute(&rule.head, &state)).is_ok());
    }
  }

  pub fn derive(&self, query : &Vec<Clause>) -> Result<Vec<Vec<Value>>, Error> {
    Ok(try!(self.fact_db.search_facts(query)))
  }

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

  pub fn reg_func(&mut self, name : String, func : HFunc) {
      self.funcs.insert(name, func);
  }
}
