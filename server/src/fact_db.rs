//Unstable is needed for Send, this may change post 1.0.0
#![allow(unstable)]

pub type PredId = u64;

use holmes_capnp::holmes;
use capnp::list::{struct_list};

pub enum PredResponse {
  PredicateCreated,
  PredicateExists,
  PredicateTypeMismatch,
  PredicateInvalid(String)
}

pub trait FactDB: Send {
  fn new_predicate<'a>(&mut self, name : &str,
                     types : struct_list::Reader<'a, holmes::h_type::Reader<'a>>)
                                        -> PredResponse;
}
