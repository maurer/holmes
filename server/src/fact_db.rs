//Unstable is needed for Send, this may change post 1.0.0
#![allow(unstable)]

pub type PredId = u64;

use holmes_capnp::holmes;
use capnp::list::{struct_list};

#[derive(Copy)]
pub enum PredResponse {
  PredicateCreated(PredId),
  PredicateExists(PredId),
  PredicateTypeMismatch,
  PredicateInvalid(&'static str)
}

pub trait FactDB: Send {
  fn new_predicate(&self, name : &str,
                   types : struct_list::Reader<holmes::h_type::Reader>)
                   -> PredResponse;
}
