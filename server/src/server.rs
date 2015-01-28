use std;
use std::vec::Vec;

use capnp::capability::{FromServer, Server};
use capnp::list::{primitive_list};
use capnp::{MallocMessageBuilder, MessageBuilder};

use capnp_rpc::capability::{InitRequest, LocalClient, WaitForContent};

use holmes_capnp::holmes;

use fact_db::FactDB;

pub struct HolmesImpl {
  fact_db : Box<FactDB + Send>
}

impl HolmesImpl {
  pub fn new(db : Box<FactDB+Send>) -> HolmesImpl {
    HolmesImpl {fact_db : db}
  }
}

impl holmes::Server for HolmesImpl {
  fn new_predicate(&mut self, mut context : holmes::NewPredicateContext) {
    context.done();
  }

  fn new_fact(&mut self, mut context : holmes::NewFactContext) {
    context.done();
  }

  fn derive_fact(&mut self, mut context : holmes::DeriveFactContext) {
    context.done();
  }

  fn new_rule(&mut self, mut context : holmes::NewRuleContext) {
    context.done();
  }
}
