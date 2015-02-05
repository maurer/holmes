use native_types::*;

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

pub trait FactDB: Send {
  fn new_predicate(&mut self, pred : Predicate) -> PredResponse;
  fn new_fact(&mut self, fact : &Fact) -> FactResponse;
}
