use fact_db::FactDB;
use std::collections::hash_map::HashMap;
use native_types::*;

pub struct Engine {
  fact_db : Box<FactDB + Send>,
  funcs   : HashMap<String, HFunc>
}

#[derive(Debug)]
pub enum Error {
  Invalid(String),
  Internal(String),
  Type(String),
  Db(String)
}
use self::Error::*;

fn substitute(clause : &Clause, ans : &Vec<HValue>) -> Fact {
  use native_types::MatchExpr::*;
  Fact {
    pred_name : clause.pred_name.clone(),
    args : clause.args.iter().map(|slot| {
      match slot {
        &Unbound       => panic!("Unbound is not allowed in substituted facts"),
        &Var(ref n)    => ans[*n as usize].clone(),
        &HConst(ref v) => v.clone()
      }
    }).collect()
  }
}

impl Engine {
  pub fn new(db : Box<FactDB + Send>) -> Engine {
    Engine {
      fact_db : db,
      funcs   : HashMap::new(),
    }
  }

  pub fn new_predicate(&mut self, pred : Predicate) -> Result<(), Error> {
    
    // Verify we have at least one argument
    if pred.types.len() == 0 {
      return Err(Invalid("Predicates must have at least one argument.".to_string()));
    }
    
    // Check for existing predicates/type issues
    match self.fact_db.get_predicate(&pred.name) {
      Some(p) => {
        if pred.types == p.types {
          ()
        } else {
          return Err(Type(format!("{:?} != {:?}", pred.types, p.types)))
        }
      }
      None => ()
    }

    // Have the FactDb persist it
    {
      use fact_db::PredResponse::*;
      match self.fact_db.new_predicate(pred) {
        PredicateInvalid(msg) => Err(Db(msg)),
        PredicateTypeMismatch => panic!("PredicateTypeMismatch should be masked against"),
        PredFail(msg) => Err(Internal(msg)),
        PredicateExists | PredicateCreated => Ok(())
      }
    }
  }

  pub fn new_fact(&mut self, fact : &Fact) -> Result<(), Error> {
    match self.fact_db.get_predicate(&fact.pred_name) {
      Some(ref pred) => {
        if (fact.args.len() != pred.types.len())
           || (!fact.args.iter().zip(pred.types.iter()).all(type_check)) {
          return Err(Type(format!("Fact ({:?}) does not match predicate ({:?})", fact, pred.types)))
        }
      }
      None => return Err(Invalid("Predicate not registered".to_string()))
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
        FactTypeMismatch => Err(Type("Fact type mismatch".to_string())),
        FactPredUnreg(_) => panic!("FactPredUnreg should be impossible"),
        FactFail(msg) => Err(Internal(msg))
      }
    }
  }

  fn eval(&self, expr : &Expr, subs : &Vec<HValue>) -> Vec<HValue> {
    use native_types::Expr::*;
    match *expr {
      EVar(var) => vec![subs[var as usize].clone()],
      EVal(ref val) => vec![val.clone()],
      EApp(ref fun_name, ref args) => {
        let arg_vals = args.iter().map(|arg_expr|{
          let v = self.eval(arg_expr, subs);
          v[0].clone()
        }).collect();
        (self.funcs[fun_name].run)(arg_vals)
      }
    }
  }

  fn run_rule(&mut self, rule : &Rule) {
    use fact_db::SearchResponse::*;
    match self.fact_db.search_facts(&rule.body) {
      SearchAns(anss) => {
        //TODO: support anything other than one match, then wheres
        'ans: for ans in anss {
          if self.fact_db.rule_cache_miss(&rule, &ans) {
            let mut ans = ans.clone();
            for where_clause in rule.wheres.iter() {
              let resp = self.eval(&where_clause.rhs, &ans);
              for (lhs, rhs) in where_clause.asgns.iter().zip(resp.iter()) {
                use native_types::MatchExpr::*;
                match *lhs {
                  Unbound   => (),
                  HConst(ref v) => {
                    if *v != *rhs {
                      continue 'ans
                    }
                  }
                  Var(n) => {
                    //Definition should be next to be defined.
                    assert!(n as usize == ans.len());
                    ans.push(rhs.clone());
                  }
                }
              }
            }
            assert!(self.new_fact(&substitute(&rule.head, &ans)).is_ok());
          }
        }
      }
      SearchInvalid(s) => panic!("Internal invalid search query {}", s),
      SearchFail(s) => panic!("Search procedure failure {}", s),
      SearchNone => ()
    }
  }

  pub fn derive(&self, query : &Vec<Clause>) -> Result<Vec<Vec<HValue>>, Error> {
    use fact_db::SearchResponse::*;
    match self.fact_db.search_facts(query) {
      SearchNone => Ok(vec![]),
      SearchAns(ans) => Ok(ans),
      SearchInvalid(err) => Err(Invalid(format!("{:?}", err))),
      SearchFail(err) => Err(Internal(format!("{:?}", err))),
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
        Err(Internal(msg))
      }
      RuleInvalid(msg) => {
        Err(Invalid(msg))
      }
    }
  }
  pub fn reg_func(&mut self, name : String, func : HFunc) {
      self.funcs.insert(name, func);
  }
}
