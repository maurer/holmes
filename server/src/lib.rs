extern crate capnp;
extern crate "capnp-rpc" as capnp_rpc;
extern crate postgres;

pub mod server;
pub mod fact_db;
pub mod pg_db;

pub mod holmes_capnp {
  include!(concat!(env!("OUT_DIR"), "/holmes_capnp.rs"));
}

mod native_types;
mod rpc_server;
pub mod server_control;
