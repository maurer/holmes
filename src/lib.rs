extern crate capnp;
extern crate capnp_rpc;
extern crate postgres;
extern crate rustc_serialize;

pub mod server;
pub mod fact_db;
pub mod pg_db;
pub mod engine;

pub mod holmes_capnp {
  include!(concat!(env!("OUT_DIR"), "/holmes_capnp.rs"));
}

pub mod native_types;
mod rpc_server;
pub mod server_control;

pub mod client;
