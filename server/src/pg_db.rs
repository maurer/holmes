use fact_db::{FactDB, PredResponse};

use holmes_capnp::holmes;
use capnp::list::{struct_list};

pub struct PgDB;

impl FactDB for PgDB {
  fn new_predicate(&self, name : String,
                   types : struct_list::Reader<holmes::h_type::Reader>)
                   -> PredResponse {
    PredResponse::PredicateInvalid("unimplemented")
  }
}
