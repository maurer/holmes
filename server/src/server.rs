use capnp::capability::Server;

use holmes_capnp::holmes;

use fact_db::FactDB;
use native_types::*;

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
    use fact_db::PredResponse::*;
    let (params, mut results) = context.get();
    let types = convert_types(params.get_arg_types());
    let predicate = Predicate {
      name  : String::from_str(params.get_pred_name()).clone(),
      types : types
    };
    match self.fact_db.new_predicate(predicate) {
        PredicateCreated
      | PredicateExists => {
          results.set_valid(true);
          context.done();
        }
        PredicateTypeMismatch => {
          results.set_valid(false);
          context.done();
        }
        PredicateInvalid(m) => {
          context.fail(m);
        }
    }
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
