//TODO: Wipe out this abstraction
//This abstraction made sense when it was possible to retarget to another db
//My dynamic typing scheme is making it so we are writing against exactly pg,
//so this is somewhat disingenuous, and by the time that it's important enough
//to replace this abstraction, I suspect the required interface will be
//different

use native_types::*;
use db_types::values::Value;
use db_types::types::Type;
use std::sync::Arc;

pub enum RuleBy {
  Pred(String)
}

pub enum PredResponse {
    PredicateCreated,
    PredicateExists,
    PredicateTypeMismatch,
    PredicateInvalid(String),
    PredFail(String)
}
pub enum FactResponse {
    FactCreated,
    FactExists,
    FactTypeMismatch,
    FactPredUnreg(String),
    FactFail(String)
}
pub enum SearchResponse {
    SearchNone,
    SearchAns(Vec<Vec<Arc<Value>>>),
    SearchInvalid(String),
    SearchFail(String)
}
pub enum RuleResponse {
    RuleFail(String),
    RuleInvalid(String),
    RuleAdded
}

pub trait FactDB: Send {
  fn get_predicate(&self, name : &str) -> Option<&Predicate>;
  fn new_predicate(&mut self, pred : &Predicate) -> PredResponse;
  fn new_fact(&mut self, fact : &Fact) -> FactResponse;
  fn search_facts(&self, query : &Vec<Clause>) -> SearchResponse;
  fn new_rule(&mut self, rule : &Rule) -> RuleResponse;
  fn rule_cache_miss(&mut self, rule : &Rule, args : &Vec<Arc<Value>>) -> bool;
  fn get_rules(&self, by : RuleBy) -> Vec<Rule>;
  fn get_type(&self, name : &str) -> Option<Arc<Type>>;
}
