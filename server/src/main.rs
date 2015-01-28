extern crate capnp;
extern crate "capnp-rpc" as capnp_rpc;

mod server;
mod fact_db;
mod pg_db;

pub mod holmes_capnp {
  include!(concat!(env!("OUT_DIR"), "/holmes_capnp.rs"));
}

use server::HolmesImpl;
use pg_db::PgDB;
use holmes_capnp::holmes;

use capnp_rpc::ez_rpc::EzRpcServer;

pub fn main () {
  let rpc_server = EzRpcServer::new("127.0.0.1:8080").unwrap();
  let holmes = Box::new(holmes::ServerDispatch {
    server : Box::new(HolmesImpl::new(Box::new(PgDB)))
    });
  rpc_server.export_cap("holmes", holmes);
  let _ = rpc_server.serve().join();
}
