use holmes_capnp::holmes;
use capnp_rpc::ez_rpc::EzRpcClient;
use std::old_io::IoResult;
use native_types::*;
use capnp_rpc::capability::{InitRequest, WaitForContent};
use std::num::{ToPrimitive, FromPrimitive};

pub struct Client {
  holmes : holmes::Client,
  pub rpc_client : EzRpcClient
}

impl Client {
  pub fn new(addr : &str) -> IoResult<Client> {
    let mut rpc_client = try!(EzRpcClient::new(addr));
    let holmes : holmes::Client = rpc_client.import_cap("holmes");
    Ok(Client {
      holmes     : holmes,
      rpc_client : rpc_client
    })
  }
  //TODO: figure out how to represent the output type for a pipelinable promise
  pub fn new_predicate(&mut self, pred : &Predicate) -> bool {
    let mut pred_req = self.holmes.new_predicate_request();
    let mut pred_data = pred_req.init();
    pred_data.set_pred_name(pred.name.as_slice());
    let type_len = pred.types.len().to_u32().unwrap();
    let mut type_data = pred_data.borrow().init_arg_types(type_len);
    for i in 0..(type_len - 1) {
      let idex : usize = FromPrimitive::from_u32(i).unwrap();
      match pred.types[idex] {
        HType::HString => {type_data.borrow().get(i).set_string(())}
        HType::Blob    => {type_data.borrow().get(i).set_blob(())}
        HType::UInt64  => {type_data.borrow().get(i).set_uint64(())}
      }
    }
    pred_req.send().wait().unwrap().get_valid()
  }

  pub fn new_fact(&mut self, fact : &Fact) -> () {
    use native_types::HValue::*;
    let mut resp = {
      let mut fact_req = self.holmes.new_fact_request();
      let req_data = fact_req.init();
      let mut fact_data = req_data.init_fact();
      fact_data.set_predicate(fact.pred_name.as_slice());
      let arg_len = fact.args.len().to_u32().unwrap();
      let mut arg_data = fact_data.borrow().init_args(arg_len);
      for (i, val) in fact.args.iter().enumerate() {
        let i = i as u32;
        match val {
          &HStringV(x) => {arg_data.borrow().get(i).set_string(x)}
          &BlobV(x)    => {arg_data.borrow().get(i).set_blob(x)}
          &UInt64V(x)  => {arg_data.borrow().get(i).set_uint64(x)}
        }
      }
      fact_req.send()
    };
    resp.wait().unwrap();
  }
}
