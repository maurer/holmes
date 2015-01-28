type PredId = u64;

use holmes_capnp::holmes;
use capnp::list::{struct_list};

pub enum PredResponse {
  PredicateCreated(PredId),
  PredicateExists(PredId),
  PredicateTypeMismatch,
  PredicateInvalid(&'static str)
}

pub trait FactDB: Send {
  fn new_predicate(&self, name : String,
                   types : struct_list::Reader<holmes::h_type::Reader>)
                   -> PredResponse;
}
