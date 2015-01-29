use capnp::capability::Server;

use holmes_capnp::holmes;

use fact_db::FactDB;
use fact_db::PredResponse;

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
    let (params, mut results) = context.get();
    let res = self.fact_db.new_predicate(params.get_pred_name(), params.get_arg_types());
    let pred_id = 
      match res {
        PredResponse::PredicateInvalid(_) => 42,
        _ => 5
      };
    results.set_pred_id(pred_id);
    context.done();
  }

  fn new_fact(&mut self, context : holmes::NewFactContext) {
    context.done();
  }

  fn derive_fact(&mut self, context : holmes::DeriveFactContext) {
    context.done();
  }

  fn new_rule(&mut self, context : holmes::NewRuleContext) {
    context.done();
  }
}
