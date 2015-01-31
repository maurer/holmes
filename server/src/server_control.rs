use pg_db::PgDB;
use capnp_rpc::ez_rpc::EzRpcServer;
use holmes_capnp::holmes;
use server::HolmesImpl;
use std::error::Error;
use postgres::{SslMode, Connection, IntoConnectParams};
use std::fmt::{Formatter, Display};
use std::thread::{Thread, JoinGuard};

pub fn unwrap<T, E : Display>(r : &Result<T,E>) -> &T {
  match r {
    &Ok(ref v) => {v}
    &Err(ref e) => {panic!(format!("unwrap failed: {}", e))}
  }
}

pub enum DB {
  Postgres(&'static str)
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

impl DB {
  fn destroy(&self) -> Result<(), Box<Error>> {
    match self {
      &DB::Postgres(str) => { 
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
      &DB::Postgres(str) => {
        let mut params = try!(str.into_connect_params());
        let old_db = try!(params.database.ok_or(NoDB));
        params.database = Some("postgres".to_string());
        let conn = try!(Connection::connect(params, &SslMode::None));
        let create_query = format!("CREATE DATABASE {}", &old_db);
        try!(conn.execute(create_query.as_slice(), &[]));
      }
    }
    Ok(())
  }
}

pub struct Server<'a> {
  addr : &'a str,
  db : DB,
  thread : Option<JoinGuard<'a, ()>>
}

impl<'a> Server<'a> {
  pub fn new(addr : &str, db : DB) -> Server {
    Server {
      addr   : addr,
      db     : db,
      thread : None
    }
  }
  pub fn boot(&mut self) -> Result<(), Box<Error+'a>> {
    self.db.create();
    let rpc_server = try!(EzRpcServer::new(self.addr));
    let db = match self.db {
      DB::Postgres(s) => {try!(PgDB::new(s))}
    };
    let holmes = Box::new(holmes::ServerDispatch {
      server : Box::new(HolmesImpl::new(Box::new(db)))
      });
    rpc_server.export_cap("holmes", holmes);
    let thread = Thread::scoped(move || {
      let _ = rpc_server.serve().join();
    });
    self.thread = Some(thread);
    Ok(())
  }
  pub fn join(&mut self) -> ::std::thread::Result<()> {
    let thread = self.thread.take();
    thread.expect("Tried to join non-running server").join()
  }
  pub fn shutdown(&mut self) -> ::std::thread::Result<()> {
    //TODO: Send message to thread that destroys it
    self.join()
  }
  pub fn destroy(&mut self) -> ::std::thread::Result<()> {
    let res = self.shutdown();
    self.db.destroy();
    res
  }
  pub fn reboot(&mut self) -> Result<(), Box<Error+'a>> {
    self.shutdown();
    try!(self.boot());
    Ok(())
  }
}
