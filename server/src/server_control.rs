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
use std::fmt::Debug;
use std::old_io::TcpStream;
use std::convert::From;
use std::borrow::ToOwned;
use std::sync::mpsc::{Sender,Receiver};

pub enum DB {
  Postgres(String)
}

pub enum ControlError {
  NoDB,
  AnyErr(Box<Error>),
  ControlIO(::std::old_io::IoError),
  PgConnect(::postgres::ConnectError),
  PgErr(::postgres::Error),
  PgDbErr(pg_db::DBError)
}

use server_control::ControlError::*;

impl Display for ControlError {
  fn fmt(&self, fmt : &mut Formatter) -> Result<(),::std::fmt::Error> {
    match *self {
      NoDB             => fmt.write_str("No database specified"),
      AnyErr(ref e)    => fmt.write_fmt(format_args!("{}", e)),
      ControlIO(ref e) => fmt.write_fmt(format_args!("{}", e)),
      PgConnect(ref e) => fmt.write_fmt(format_args!("{}", e)),
      PgErr(ref e)     => fmt.write_fmt(format_args!("{}", e)),
      PgDbErr(ref e)   => fmt.write_fmt(format_args!("{}", e)),
    }
  }
}

impl Debug for ControlError  {
  fn fmt(&self, fmt : &mut Formatter) -> Result<(),::std::fmt::Error> {
    match *self {
      NoDB             => fmt.write_str("NoDB"),
      AnyErr(ref e)    => fmt.write_fmt(format_args!("AnyErr({:?})", e)),
      ControlIO(ref e) => fmt.write_fmt(format_args!("ControlIO({:?})", e)),
      PgConnect(ref e) => fmt.write_fmt(format_args!("PgConnect({:?})", e)),
      PgErr(ref e)     => fmt.write_fmt(format_args!("PgErr({:?})", e)),
      PgDbErr(ref e)   => fmt.write_fmt(format_args!("PgDbErr({:?})", e))
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

impl From<::postgres::ConnectError> for ControlError {
  fn from(ce : ::postgres::ConnectError) -> ControlError {PgConnect(ce)}
}

impl From<::postgres::Error> for ControlError {
  fn from(e : ::postgres::Error) -> ControlError {PgErr(e)}
}

impl From<::std::old_io::IoError> for ControlError {
  fn from(e : ::std::old_io::IoError) -> ControlError {ControlIO(e)}
}

impl From<pg_db::DBError> for ControlError {
  fn from(e : pg_db::DBError) -> ControlError {PgDbErr(e)}
}

impl<'a> DB {
  fn destroy(&self) -> Result<(), Box<Error>> {
    match self {
      &DB::Postgres(ref str) => { 
        let mut params = try!(str.into_connect_params());
        let old_db = try!(params.database.ok_or(NoDB));
        params.database = Some("postgres".to_owned());
        let conn = try!(Connection::connect(params, &SslMode::None));
        let disco_query = format!("SELECT pg_terminate_backend(pg_stat_activity.pid) FROM pg_stat_activity WHERE pg_stat_activity.datname = '{}' AND pid <> pg_backend_pid()", &old_db);
        try!(conn.execute(disco_query.as_slice(), &[]));
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
        params.database = Some("postgres".to_owned());
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
  thread  : Option<JoinGuard<'a, ()>>,
  control : Option<Sender<Command>>,
  status  : Option<Receiver<Status>>
}

impl<'a> Server<'a> {
  pub fn new(addr : &str, db : DB) -> Server {
    Server {
      addr    : addr,
      db      : db,
      thread  : None,
      control : None,
      status  : None
    }
  }
  pub fn boot(&mut self) -> Result<(), Box<Error>> {
    try!(self.db.create());
    let (rpc_server, control, status) = try!(RpcServer::new(self.addr));
    let db = match self.db {
      DB::Postgres(ref s) => {try!(PgDB::new(s.as_slice()))}
    };
    let holmes = Box::new(holmes::ServerDispatch {
      server : Box::new(HolmesImpl::new(Box::new(db)))
      });
    self.control = Some(control);
    self.status  = Some(status);
    self.thread = Some(rpc_server.serve(holmes));
    Ok(())
  }
  pub fn join(&mut self) -> () {
    let thread = self.thread.take();
    thread.expect("Tried to join non-running server").join()
  }
  pub fn shutdown(&mut self) -> Result<(), Box<Error>> {
    try!(self.control
             .take()
             .expect("No control channel held.")
             .send(Command::Shutdown));
    try!(TcpStream::connect(self.addr).map_err(ControlIO));
    let s = try!(self.status
                     .take()
                     .expect("No status channel held.")
                     .recv());
    assert_eq!(s, Status::Offline);
    self.join();
    Ok(())
  }
  pub fn destroy(&mut self) -> Result<(), Box<Error>> {
    try!(self.shutdown());
    self.db.destroy()
  }
  pub fn reboot(&mut self) -> Result<(), Box<Error>> {
    try!(self.shutdown());
    try!(self.boot());
    Ok(())
  }
}
