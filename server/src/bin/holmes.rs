extern crate "holmes" as holmes_crate;
extern crate capnp;
extern crate "capnp-rpc" as capnp_rpc;

use holmes_crate::pg_db::PgDB;
use capnp_rpc::ez_rpc::EzRpcServer;
use holmes_crate::holmes_capnp::holmes;
use holmes_crate::server::HolmesImpl;

pub fn main () {
  let rpc_server = EzRpcServer::new("127.0.0.1:8080").unwrap();
  let pg_db = PgDB::new("postgresql://maurer@localhost").unwrap();
  let holmes = Box::new(holmes::ServerDispatch {
    server : Box::new(HolmesImpl::new(Box::new(pg_db)))
    });
  rpc_server.export_cap("holmes", holmes);
  let _ = rpc_server.serve().join();
}
