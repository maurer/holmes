use pg_db::PgDB;
use pg_db;
use holmes_capnp::holmes;
use server::HolmesImpl;
use std::error::Error;
use postgres::{SslMode, Connection, IntoConnectParams};
use std::fmt::{Formatter, Display};
use std::thread::JoinGuard;
use rpc_server::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::old_io::TcpStream;
use std::error::FromError;

pub fn unwrap<T, E : Display>(r : &Result<T,E>) -> &T {
  match r {
    &Ok(ref v) => {v}
    &Err(ref e) => {panic!(format!("unwrap failed: {}", e))}
  }
}

pub enum DB {
  Postgres(String)
}

pub enum ControlError {
  NoDB,
  AnyErr(Box<::std::any::Any + Send>),
  ControlIO(::std::old_io::IoError),
  PgConnect(::postgres::ConnectError),
  PgErr(::postgres::Error),
  PgDbErr(pg_db::DBError)
}

use server_control::ControlError::*;

impl Display for ControlError {
  fn fmt(&self, fmt : &mut Formatter) -> Result<(),::std::fmt::Error> {
    match self {
      &NoDB              => fmt.write_str("No database specified"),
      &AnyErr(_)         => fmt.write_str("Error from thread"),
      &ControlIO(ref io) => io.fmt(fmt),
      &PgConnect(ref e)  => e.fmt(fmt),
      &PgErr(ref e)      => e.fmt(fmt),
      &PgDbErr(ref e)    => e.fmt(fmt),
    }
  }
}

impl Error for ControlError {
  fn description(&self) -> &str {
    match self {
      &NoDB              => "No database specified",
      &AnyErr(_)         => "Error from thread",
      &ControlIO(ref io) => io.description(),
      &PgConnect(ref e)  => e.description(),
      &PgErr(ref e)      => e.description(),
      &PgDbErr(ref e)    => e.description()
    }
  }
} 

impl FromError<::postgres::ConnectError> for ControlError {
  fn from_error(ce : ::postgres::ConnectError) -> ControlError {PgConnect(ce)}
}

impl FromError<::postgres::Error> for ControlError {
  fn from_error(e : ::postgres::Error) -> ControlError {PgErr(e)}
}

impl FromError<::std::old_io::IoError> for ControlError {
  fn from_error(e : ::std::old_io::IoError) -> ControlError {ControlIO(e)}
}

impl FromError<pg_db::DBError> for ControlError {
  fn from_error(e : pg_db::DBError) -> ControlError {PgDbErr(e)}
}

impl<'a> DB {
  fn destroy(&self) -> Result<(), ControlError> {
    match self {
      &DB::Postgres(ref str) => { 
        let mut params = try!(str.into_connect_params());
        let old_db = try!(params.database.ok_or(NoDB));
        params.database = Some("postgres".to_string());
        let conn = try!(Connection::connect(params, &SslMode::None));
        let drop_query = format!("DROP DATABASE {}", &old_db);
        try!(conn.execute(drop_query.as_slice(), &[]));
      }
    }
    Ok(())
  }
  fn create(&self) -> Result<(), ControlError> {
    match self {
      &DB::Postgres(ref str) => {
        let mut params = try!(str.into_connect_params());
        let old_db = try!(params.database.ok_or(NoDB));
        params.database = Some("postgres".to_string());
        let conn = try!(Connection::connect(params, &SslMode::None));
        let create_query = format!("CREATE DATABASE {}", &old_db);
        let _ = conn.execute(create_query.as_slice(), &[]);
      }
    }
    Ok(())
  }
}

pub struct Server<'a> {
  addr : &'a str,
  db : DB,
  thread : Option<JoinGuard<'a, ()>>,
  shutdown : Option<Arc<AtomicBool>>
}

impl<'a> Server<'a> {
  pub fn new(addr : &str, db : DB) -> Server {
    Server {
      addr     : addr,
      db       : db,
      thread   : None,
      shutdown : None
    }
  }
  pub fn boot(&mut self) -> Result<(), ControlError> {
    try!(self.db.create());
    let rpc_server = try!(RpcServer::new(self.addr));
    let db = match self.db {
      DB::Postgres(ref s) => {try!(PgDB::new(s.as_slice()))}
    };
    let holmes = Box::new(holmes::ServerDispatch {
      server : Box::new(HolmesImpl::new(Box::new(db)))
      });
    rpc_server.export_cap("holmes", holmes);
    self.shutdown.clone_from(&Some(rpc_server.shutdown.clone()));
    self.thread = Some(rpc_server.serve());
    Ok(())
  }
  pub fn join(&mut self) -> ::std::thread::Result<()> {
    let thread = self.thread.take();
    thread.expect("Tried to join non-running server").join()
  }
  pub fn shutdown(&mut self) -> Result<(), ControlError> {
    let shutdown = self.shutdown.take();
    shutdown.expect("Tried to shut down non-running server").store(true,Ordering::Release);
    try!(TcpStream::connect(self.addr).map_err(ControlIO));
    self.join().map_err(AnyErr)
  }
  pub fn destroy(&mut self) -> Result<(), ControlError> {
    try!(self.shutdown());
    self.db.destroy()
  }
  pub fn reboot(&mut self) -> Result<(), ControlError> {
    try!(self.shutdown());
    try!(self.boot());
    Ok(())
  }
}
