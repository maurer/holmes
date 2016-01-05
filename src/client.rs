use holmes_capnp::holmes;
use capnp_rpc::ez_rpc::EzRpcClient;
use native_types::*;
use capnp_rpc::capability::{InitRequest, LocalClient, WaitForContent};
use std::borrow::ToOwned;

pub struct Client {
  holmes : holmes::Client,
  pub rpc_client : EzRpcClient
}

struct Func {
  h_func : HFunc
}

impl Func {
  pub fn new(func : HFunc) -> Func {
    Func {h_func : func}
  }
}

impl holmes::h_func::Server for Func {
  fn types(&mut self, mut context : holmes::h_func::TypesContext) {
    {
      let (_, mut results) = context.get();
      {
        let input_len = self.h_func.input_types.len() as u32;
        let mut inputs = results.borrow().init_input_types(input_len);
        for i in 0..input_len {
          capnp_type(inputs.borrow().get(i),
                     &self.h_func.input_types[i as usize])
        }
      }
      {
        let output_len = self.h_func.output_types.len() as u32;
        let mut outputs = results.borrow().init_output_types(output_len);
        for i in 0..output_len {
          capnp_type(outputs.borrow().get(i),
                     &self.h_func.output_types[i as usize])
        }
      }
    }
    context.done()
  }
  fn run(&mut self, mut context : holmes::h_func::RunContext) {
    {
      let (params, results) = context.get();
      let ins  = convert_vals(params.get_args().unwrap());
      let outs = (self.h_func.run)(ins);
      let mut res_data = results.init_results(outs.len() as u32);
      for (i, v) in outs.iter().enumerate() {
        capnp_val(res_data.borrow().get(i as u32), v)
      }
    }
    context.done()
  }
}

impl Client {
  pub fn new(addr : &str) -> Result<Client,::std::io::Error> {
    let mut rpc_client = try!(EzRpcClient::new(addr));
    let holmes : holmes::Client = rpc_client.get_main();
    Ok(Client {
      holmes     : holmes,
      rpc_client : rpc_client
    })
  }
  //TODO: figure out how to represent the output type for a pipelinable promise
  pub fn new_predicate(&mut self, pred : &Predicate) -> Result<(), String> {
    let mut pred_req = self.holmes.new_predicate_request();
    {
      let mut pred_data = pred_req.init();
      pred_data.set_pred_name(&pred.name);
      let type_len = pred.types.len() as u32;
      let mut type_data = pred_data.borrow().init_arg_types(type_len);
      for i in 0..type_len {
        let idex : usize = i as usize;
        capnp_type(type_data.borrow().get(i), &pred.types[idex])
      }
    }
    pred_req.send().wait().map(|_|{()})
  }

  pub fn new_fact(&mut self, fact : &Fact) -> Result<(), String> {
    let mut resp = {
      let mut fact_req = self.holmes.new_fact_request();
      {
        let req_data = fact_req.init();
        let mut fact_data = req_data.init_fact();
        fact_data.set_predicate(&fact.pred_name);
        let arg_len = fact.args.len() as u32;
        let mut arg_data = fact_data.borrow().init_args(arg_len);
        for (i, val) in fact.args.iter().enumerate() {
          let i = i as u32;
          capnp_val(arg_data.borrow().get(i), val);
        }
      }
      fact_req.send()
    };
    resp.wait().map(|_|{()})
  }
  pub fn derive(&mut self, query : Vec<&Clause>) ->
    Result<Vec<Vec<HValue>>, ::capnp::Error> {
    let mut resp = {
      let mut derive_req = self.holmes.derive_request();
      {
        let mut query_data = derive_req.init().init_query(query.len() as u32);
        for (i, clause) in query.iter().enumerate() {
          let i = i as u32;
          capnp_clause(query_data.borrow().get(i), clause);
        }
      }
      derive_req.send()
    };
    let resp_data = resp.wait().unwrap();
    let ctxs = try!(resp_data.get_ctx());
    let mut anss = Vec::new();
    for ctx in ctxs.iter() {
      let mut ans = Vec::new();
      for asgn in ctx.unwrap().iter() {
        ans.push(convert_val(asgn).to_owned());
      }
      anss.push(ans);
    }
    Ok(anss)
  }
  pub fn new_rule(&mut self, rule : &Rule) ->
    Result<(), String> {
    let mut resp = {
      let mut rule_req = self.holmes.new_rule_request();
      {
        let rule_data = rule_req.init().init_rule();
        capnp_rule(rule_data, rule);
      }
      rule_req.send()
    };
    resp.wait().unwrap();
    Ok(())
  }
  pub fn new_func(&mut self, name : &str, func : HFunc) ->
    Result<(), String> {
    let func = Func::new(func);
    let mut resp = {
      let mut func_req = self.holmes.new_func_request();
      {
        let mut func_data = func_req.init();
        func_data.set_name(name);
        func_data.set_func(
          holmes::h_func::ToClient::new(func).from_server::<LocalClient>()); //TODO find out what from_server does
        //Set stuff here
      }
      func_req.send()
    };
    resp.wait().unwrap();
    Ok(())
  }
}

#[macro_export]
macro_rules! htype {
  (string) => { ::holmes::native_types::HType::HString};
  (blob  ) => { ::holmes::native_types::HType::Blob};
  (uint64) => { ::holmes::native_types::HType::UInt64};
}

#[macro_export]
macro_rules! client_exec {
  ($client:ident, { $( $action:expr );* }) => {
      { $( $action($client).unwrap() );* }
  };
}

#[macro_export]
macro_rules! predicate {
  ($client:ident, $pred_name:ident($($t:ident),*)) => {
    $client.new_predicate(&Predicate {
      name  : stringify!($pred_name).to_string(),
      types : vec![$(htype!($t),)*]
    })
  };
  ($pred_name:ident($($t:ident),*)) => { |client : &mut Client| {
    let res : Result<(), String> = predicate!(client, $pred_name($($t),*));
    res
  }};
}

#[macro_export]
macro_rules! fact {
  ($client:ident, $pred_name:ident($($a:expr),*)) => {
    $client.new_fact(&Fact {
      pred_name : stringify!($pred_name).to_string(),
      args : vec![$($a.to_hvalue()),*]
    })
  };
  ($pred_name:ident($($a:expr),*)) => { |client : &mut Client| {
    let res : Result<(), String> = fact!(client, $pred_name($($a),*));
    res
  }};
}

#[macro_export]
macro_rules! clause_match {
  ($vars:ident, $n:ident, [_]) => { ::holmes::native_types::MatchExpr::Unbound };
  ($vars:ident, $n:ident, ($v:expr)) => { ::holmes::native_types::MatchExpr::HConst($v.to_hvalue()) };
  ($vars:ident, $n:ident, $m:ident) => {{
    use std::collections::hash_map::Entry::*;
    use ::holmes::native_types::MatchExpr::*;
    match $vars.entry(stringify!($m).to_string()) {
      Occupied(entry) => Var(*entry.get()),
      Vacant(entry) => {
        $n = $n.wrapping_add(1);
        entry.insert($n);
        Var($n)
      }
    }
  }};
}

#[macro_export]
macro_rules! derive {
  ($client:ident, $($pred_name:ident($($m:tt),*))&*) => {{
    use std::collections::HashMap;
    let mut vars : HashMap<String, u32> = HashMap::new();
    let mut n : u32 = 0xffffffff;
    $client.derive(vec![$(&Clause {
      pred_name : stringify!($pred_name).to_string(),
      args : vec![$(clause_match!(vars, n, $m)),*]
    }),*])
  }}
}

pub fn var_to_evar(var : MatchExpr) -> Expr {
  match var {
    MatchExpr::Var(var_no) => Expr::EVar(var_no),
    x => panic!("var_to_evar was passed nonvar: {:?}", x)
  }
}

#[macro_export]
macro_rules! hexpr {
  ($vars:ident, $n:ident, [$hexpr_name:ident]) => {
    var_to_evar(clause_match!($vars, $n, $hexpr_name))
  };
  ($vars:ident, $n:ident, ($hexpr:expr)) => {
    Expr::EVal($hexpr.to_hvalue())
  };
  ($vars:ident, $n:ident, {$hexpr_func:ident($($hexpr_arg:tt),*)}) => {
    Expr::EApp(stringify!($hexpr_func).to_string(), vec![$(hexpr!($vars, $n, $hexpr_arg)),*])
  };
}

#[macro_export]
macro_rules! rule {
  ($client:ident, $($head_name:ident($($m:tt),*)),* <= $($body_name:ident($($mb:tt),*))&*,
   {$(let $($bind:tt),* = $hexpr:tt);*}) => {{
    use std::collections::HashMap;
    let mut vars : HashMap<String, u32> = HashMap::new();
    let mut n : u32 = 0xffffffff;
    $client.new_rule(&Rule {
      body : vec![$(Clause {
        pred_name : stringify!($body_name).to_string(),
        args : vec![$(clause_match!(vars, n, $mb)),*]
      }),*],
      head : vec![$(Clause {
        pred_name : stringify!($head_name).to_string(),
        args : vec![$(clause_match!(vars, n, $m)),*]
      }),*],
      wheres : vec! [$(WhereClause {
        asgns : vec![$(::holmes::native_types::BindExpr::Normal(clause_match!(vars, n, $bind))),*],
        rhs   : hexpr!(vars, n, $hexpr)
      }),*]
    })
  }};
  ($($head_name:ident($($m:tt),*)),* <= $($body_name:ident($($mb:tt),*))&*) => {
    |client : &mut Client| {
      rule!(client, $($head_name($($m),*)),* <= $($body_name($($mb),*))&*, {})
    }
  };
  ($($head_name:ident($($m:tt),*)),* <= $($body_name:ident($($mb:tt),*))&*, {$(let $($bind:tt),* = $hexpr:tt);*}) => {
    |client : &mut Client| {
      rule!(client, $($head_name($($m),*)),* <= $($body_name($($mb),*))&*, {$(let $($bind),* = $hexpr);*})
    }
  };

}

#[macro_export]
macro_rules! func {
  ($client:ident, let $name:ident : [$($src:ident),*] -> [$($dst:ident),*] = $body:expr) => {
    $client.new_func(stringify!($name), HFunc {
      input_types  : vec![$(htype!($src)),*],
      output_types : vec![$(htype!($dst)),*],
      run : Box::new($body)
    })
  };
  (let $name:ident : [$($src:ident),*] -> [$($dst:ident),*] = $body:expr) => {
    |client : &mut Client| {
      func!(client, let $name : [$($src),*] -> [$($dst),*] = $body)
    }
  };
}
