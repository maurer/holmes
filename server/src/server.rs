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
        PredicateInvalid(m)
      | PredFail(m) => {
          context.fail(m);
        }
    }
  }
  
  fn new_fact(&mut self, mut context : holmes::NewFactContext) {
    use fact_db::FactResponse::*;
    let (params, _) = context.get();
    let fact_data = params.get_fact();
    let fact = Fact {
      pred_name : fact_data.get_predicate().to_string(),
      args : convert_vals(fact_data.get_args())
    };
    match self.fact_db.new_fact(&fact) {
        FactCreated
      | FactExists => context.done(),
        FactTypeMismatch =>
          context.fail("Type mismatch".to_string()),
        FactPredUnreg(s) => context.fail(
          format!("Predicate not registered: {}", s)),
        FactFail(s) => context.fail(
          format!("Internal error: {}", s))
    }
  }

  fn derive(&mut self, mut context : holmes::DeriveContext) {
    use fact_db::SearchResponse::*;
    let (params, result) = context.get();
    let clauses = convert_clauses(params.get_query());
    match self.fact_db.search_facts(clauses) {
      SearchNone => context.done(),
      SearchInvalid(s) => context.fail(
        format!("Search query invalid: {}", s)),
      SearchFail(s) => context.fail(
        format!("Internal error: {}", s)),
      SearchAns(answer_set) => {
        let mut ctxs_data = result.init_ctx(answer_set.len() as u32);
        for (i, answer) in answer_set.iter().enumerate() {
          let i = i as u32;
          let mut ctx_data = ctxs_data.borrow().init(i, answer.len() as u32);
          for (j, asgn) in answer.iter().enumerate() {
            let j = j as u32;
            capnp_val(ctx_data.borrow().get(j),
                      &disown(asgn));
          }
        }
        context.done();
      }
    }
  }

  fn new_rule(&mut self, context : holmes::NewRuleContext) {
    context.done();
  }
}
