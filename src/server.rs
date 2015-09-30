use capnp::capability::Server;

use holmes_capnp::holmes;

use fact_db::FactDB;
use native_types::*;
use engine::Engine;

pub struct HolmesImpl {
  engine : Engine
}

impl HolmesImpl {
  pub fn new(db : Box<FactDB+Send>) -> HolmesImpl {
    HolmesImpl {engine : Engine::new(db)}
  }
}

impl holmes::Server for HolmesImpl {
  fn new_predicate(&mut self, mut context : holmes::NewPredicateContext) {
    let predicate = {
      let (params, _) = context.get();
      let types = convert_types(params.get_arg_types().unwrap());
      Predicate {
        name  : params.get_pred_name().unwrap().to_string().clone(),
        types : types
      }
    };
    match self.engine.new_predicate(predicate) {
      Ok(()) => {
        context.done();
      }
      Err(e) => {
        context.fail(format!("{:?}", e));
      }
    }
  }

  fn new_fact(&mut self, mut context : holmes::NewFactContext) {
    let fact = {
      let (params, _) = context.get();
      let fact_data = params.get_fact().unwrap();
      Fact {
        pred_name : fact_data.get_predicate().unwrap().to_string(),
        args : convert_vals(fact_data.get_args().unwrap())
      }
    };
    match self.engine.new_fact(&fact) {
      Ok(()) => context.done(),
      Err(e) => context.fail(format!("{:?}", e))
    }
  }

  fn derive(&mut self, mut context : holmes::DeriveContext) {
    let clauses = {
      let (params, _) = context.get();
      convert_clauses(params.get_query().unwrap())
    };
    match self.engine.derive(&clauses) {
      Ok(answer_set) => {
        {
          let (_, result) = context.get();
          let mut ctxs_data = result.init_ctx(answer_set.len() as u32);
          for (i, answer) in answer_set.iter().enumerate() {
            let i = i as u32;
            let mut ctx_data = ctxs_data.borrow().init(i, answer.len() as u32);
            for (j, asgn) in answer.iter().enumerate() {
              let j = j as u32;
              capnp_val(ctx_data.borrow().get(j),
                        asgn);
            }
          }
        }
        context.done();
      }
      Err(e) => context.fail(format!("{:?}", e))
    }
  }

  fn new_rule(&mut self, mut context : holmes::NewRuleContext) {
    let rule = {
      let (params, _) = context.get();
      convert_rule(params.get_rule().unwrap())
    };
    match self.engine.new_rule(&rule) {
      Err(e) => context.fail(format!("{:?}", e)),
      Ok(()) => context.done()
    }
  }

  fn new_func(&mut self, mut context : holmes::NewFuncContext) {
    use capnp_rpc::capability::WaitForContent;
    let types = {
      let (params, _) = context.get();
      let func = params.get_func().unwrap();
      let mut type_resp = func.types_request().send();
      match type_resp.wait() {
        Ok(v) => Ok((convert_types(v.get_input_types().unwrap()),
                     convert_types(v.get_output_types().unwrap()))),
        Err(e) => Err(e)
      }
    };
    let (input_types, output_types) = match types {
      Ok(v) => v,
      Err(e) => {
        context.fail(format!("Type request failed: {}", e));
        return
      }
    };
    //TODO error relief path
    let (name, run) = {
      let (params, _) = context.get();
      let name = params.get_name().unwrap().to_string();
      let func = params.get_func().unwrap();
      (name, move |v : Vec<HValue>| {
        use capnp_rpc::capability::InitRequest;
        let mut req = func.run_request();
        {
          let mut req_data = req.init().init_args(v.len() as u32);
          for (i, v) in v.iter().enumerate() {
            capnp_val(req_data.borrow().get(i as u32), v)
          }
        }
        convert_vals(req.send().wait().unwrap().get_results().unwrap())
      })
    };
    let h_func = HFunc {
      input_types : input_types.clone(),
      output_types : output_types.clone(),
      run : Box::new(run)
    };
    self.engine.reg_func(name, h_func);
    context.done();
  }
}
