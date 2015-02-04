use native_types::*;

pub enum PredResponse {
  PredicateCreated,
  PredicateExists,
  PredicateTypeMismatch,
  PredicateInvalid(String)
}

pub trait FactDB: Send {
  fn new_predicate(&mut self, pred : Predicate) -> PredResponse;
}
