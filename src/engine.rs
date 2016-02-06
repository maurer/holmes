use fact_db::FactDB;
use std::collections::hash_map::HashMap;
use native_types::*;
use db_types::values::Value;
use db_types::types::Type;
use db_types::values;
use std::sync::Arc;

pub struct Engine {
  fact_db : Box<FactDB>,
  funcs   : HashMap<String, HFunc>
}

#[derive(Debug)]
pub enum Error {
  Invalid(String),
  Internal(String),
  Type(String),
  Db(String)
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

fn substitute(clause : &Clause, ans : &Vec<Arc<Value>>) -> Fact {
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
  pub fn new(db : Box<FactDB>) -> Engine {
    Engine {
      fact_db : db,
      funcs   : HashMap::new(),
    }
  }
  pub fn get_type(&self, name : &str) -> Option<Arc<Type>> {
    self.fact_db.get_type(name)
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

    // Have the FactDb persist it
    {
      use fact_db::PredResponse::*;
      match self.fact_db.new_predicate(pred) {
        PredicateInvalid(msg) => Err(Error::Db(msg)),
        PredicateTypeMismatch => panic!("PredicateTypeMismatch should be masked against"),
        PredFail(msg) => Err(Error::Internal(msg)),
        PredicateExists | PredicateCreated => Ok(())
      }
    }
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
      use fact_db::FactResponse::*;
      use fact_db::RuleBy;
      match self.fact_db.new_fact(&fact) {
        FactCreated => {
          for rule in self.fact_db.get_rules(RuleBy::Pred(fact.pred_name.clone())) {
            self.run_rule(&rule);
          }
          Ok(())
        }
        FactExists => Ok(()),
        FactTypeMismatch => Err(Error::Type("Fact type mismatch".to_string())),
        FactPredUnreg(_) => panic!("FactPredUnreg should be impossible"),
        FactFail(msg) => Err(Error::Internal(msg))
      }
    }
  }

  fn bind(&self, lhs : &BindExpr, rhs : Arc<Value>, state : &Vec<Arc<Value>>) -> Vec<Vec<Arc<Value>>> {
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
        let rhss = match rhs.get().downcast_ref::<Vec<Arc<Value>>>() {
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
        let rhss = match rhs.get().downcast_ref::<Vec<Arc<Value>>>() {
          Some(ref rhss) => rhss.iter(),
          _ => panic!("Attempted to destructure non-list")
        };
        rhss.flat_map(|rhs| {
          self.bind(inner, rhs.clone(), &state)
        }).collect()
      }
    }
  }

  fn eval(&self, expr : &Expr, subs : &Vec<Arc<Value>>) -> Arc<Value> {
    use native_types::Expr::*;
    match *expr {
      EVar(var) => subs[var as usize].clone(),
      EVal(ref val) => val.clone(),
      EApp(ref fun_name, ref args) => {
        let arg_vals : Vec<Arc<Value>> = args.iter().map(|arg_expr|{
          self.eval(arg_expr, subs)
        }).collect();
        let arg = if arg_vals.len() == 1 {
          arg_vals[0].clone()
        } else {
          Arc::new(values::Tuple::new(arg_vals)) as Arc<Value>
        };
        (self.funcs[fun_name].run)(arg)
      }
    }
  }


  fn run_rule(&mut self, rule : &Rule) {
    use fact_db::SearchResponse::*;
    match self.fact_db.search_facts(&rule.body) {
      SearchAns(anss) => {
        let mut states : Vec<Vec<Arc<Value>>> =
            anss.iter()
                .filter(|ans| {self.fact_db.rule_cache_miss(&rule, &ans)})
                .map(|ans| {ans.clone()})
                .collect();

        for where_clause in rule.wheres.iter() {
          let mut next_states : Vec<Vec<Arc<Value>>> = Vec::new();
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
      SearchInvalid(s) => panic!("Internal invalid search query {}", s),
      SearchFail(s) => panic!("Search procedure failure {}", s),
      SearchNone => ()
    }
  }

  pub fn derive(&self, query : &Vec<Clause>) -> Result<Vec<Vec<Arc<Value>>>, Error> {
    use fact_db::SearchResponse::*;
    match self.fact_db.search_facts(query) {
      SearchNone => Ok(vec![]),
      SearchAns(ans) => Ok(ans),
      SearchInvalid(err) => Err(Error::Invalid(format!("{:?}", err))),
      SearchFail(err) => Err(Error::Internal(format!("{:?}", err))),
    }
  }

  pub fn new_rule(&mut self, rule : &Rule) -> Result<(), Error> {
    use fact_db::RuleResponse::*;
    match self.fact_db.new_rule(rule) {
      RuleAdded => {
        self.run_rule(rule);
        Ok(())
      }
      RuleFail(msg) => {
        Err(Error::Internal(msg))
      }
      RuleInvalid(msg) => {
        Err(Error::Invalid(msg))
      }
    }
  }
  pub fn reg_func(&mut self, name : String, func : HFunc) {
      self.funcs.insert(name, func);
  }
}
