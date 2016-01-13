use native_types::*;

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
    SearchAns(Vec<Vec<HValue>>),
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
  fn rule_cache_miss(&mut self, rule : &Rule, args : &Vec<HValue>) -> bool;
  fn get_rules(&self, by : RuleBy) -> Vec<Rule>;
}
