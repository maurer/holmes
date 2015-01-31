use holmes_capnp::holmes;
use capnp_rpc::ez_rpc::EzRpcClient;
use std::old_io::IoResult;
use native_types::*;
use capnp_rpc::capability::{InitRequest, LocalClient, WaitForContent};
use std::num::{ToPrimitive, FromPrimitive};

pub struct Client {
  rpc_client : EzRpcClient,
  holmes : holmes::Client
}

impl Client {
  pub fn new(addr : &str) -> IoResult<Client> {
    let mut rpc_client = try!(EzRpcClient::new(addr));
    let holmes : holmes::Client = rpc_client.import_cap("holmes");
    Ok(Client {
      rpc_client : rpc_client,
      holmes     : holmes
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
}
