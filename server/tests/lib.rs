extern crate "holmes" as lib;
extern crate "capnp-rpc" as capnp_rpc;

use capnp_rpc::ez_rpc::EzRpcServer;
use capnp_rpc::ez_rpc::EzRpcClient;
use lib::holmes_capnp::holmes;
use lib::server::HolmesImpl;
use lib::pg_db::PgDB;
use capnp_rpc::capability::{InitRequest, LocalClient, WaitForContent};

use std::thread::Thread;

fn new_server() -> Thread {
  let addr = "127.0.0.1:8080";
  let pg_db = PgDB::new("postgresql://maurer@localhost").unwrap();
  //Deploy server
  let rpc_server = EzRpcServer::new(addr).unwrap();
  let holmes = Box::new(holmes::ServerDispatch {
      server : Box::new(HolmesImpl::new(Box::new(pg_db)))
          });
  rpc_server.export_cap("holmes", holmes);
  let serve_thread = rpc_server.serve();
  Thread::spawn(move || {
    serve_thread.join();
  })
}

#[test]
pub fn dummy_rpc_check() {
  let server_thread = new_server();
  let addr = "127.0.0.1:8080";
  //Make request
  let mut rpc_client = EzRpcClient::new(addr).unwrap();
  let holmes_client : holmes::Client = rpc_client.import_cap("holmes");
  let mut request = holmes_client.new_predicate_request();
  let mut reqI = request.init();
  reqI.set_pred_name("test_pred");
  let mut type_list = reqI.borrow().init_arg_types(2);
  type_list.borrow().get(0).set_string(());
  type_list.borrow().get(1).set_uint64(());
  let mut pred_promise = request.send();
  let pred_valid = pred_promise.wait().unwrap();
  assert_eq!(pred_valid.get_valid(), true);
}
