use pg_db::PgDB;
use holmes_capnp::holmes;
use server::HolmesImpl;
use std::error::Error;
use postgres::{SslMode, Connection, IntoConnectParams};
use std::fmt::{Formatter, Display};
use std::thread::{Thread, JoinGuard};
use rpc_server::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::old_io::TcpStream;

pub fn unwrap<T, E : Display>(r : &Result<T,E>) -> &T {
  match r {
    &Ok(ref v) => {v}
    &Err(ref e) => {panic!(format!("unwrap failed: {}", e))}
  }
}

pub enum DB {
  Postgres(String)
}

pub enum DBError {
  NoDB
}

use server_control::DBError::*;

impl Display for DBError {
  fn fmt(&self, fmt : &mut Formatter) -> Result<(),::std::fmt::Error> {
    match self {
      &NoDB => {fmt.write_str("No database specified"); Ok(())}
    }
  }
}

impl Error for DBError {
  fn description(&self) -> &str {
    match self {
      &NoDB => {"No database specified"}
    }
  }
} 

impl<'a> DB {
  fn destroy(&self) -> Result<(), Box<Error>> {
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
  fn create(&self) -> Result<(), Box<Error>> {
    match self {
      &DB::Postgres(ref str) => {
        let mut params = try!(str.into_connect_params());
        let old_db = try!(params.database.ok_or(NoDB));
        params.database = Some("postgres".to_string());
        let conn = try!(Connection::connect(params, &SslMode::None));
        let create_query = format!("CREATE DATABASE {}", &old_db);
        conn.execute(create_query.as_slice(), &[]);
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
  pub fn boot(&mut self) -> Result<(), Box<Error>> {
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
  pub fn shutdown(&mut self) -> ::std::thread::Result<()> {
    let shutdown = self.shutdown.take();
    shutdown.expect("Tried to shut down non-running server").store(true,Ordering::Release);
    TcpStream::connect(self.addr);
    self.join()
  }
  pub fn destroy(&mut self) -> ::std::thread::Result<()> {
    let res = self.shutdown();
    self.db.destroy();
    res
  }
  pub fn reboot(&mut self) -> Result<(), Box<Error>> {
    self.shutdown();
    try!(self.boot());
    Ok(())
  }
}
