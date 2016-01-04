use pg_db::PgDB;
use pg_db;
use holmes_capnp::holmes;
use server::HolmesImpl;
use std::error::Error;
use postgres::{SslMode, Connection, IntoConnectParams};
use std::fmt::{Formatter, Display};
use std::thread::JoinHandle;
use rpc_server::*;
use std::fmt::Debug;
use std::net::TcpStream;
use std::convert::From;
use std::borrow::ToOwned;
use std::sync::mpsc::{Sender, Receiver, RecvError, SendError};

pub enum DB {
  Postgres(String)
}

pub enum ControlError {
  NoDB,
  PgConnect(::postgres::error::ConnectError),
  PgErr(::postgres::error::Error),
  PgDbErr(pg_db::DBError),
  ProcessControlSend(SendError<Command>),
  ProcessControlRecv(RecvError),
  IOErr(::std::io::Error),
  PgConnectStr(Box<Error + Send + Sync>)
}

use server_control::ControlError::*;

impl Display for ControlError {
  fn fmt(&self, fmt : &mut Formatter) -> Result<(),::std::fmt::Error> {
    match *self {
      NoDB             => fmt.write_str("No database specified"),
      ProcessControlSend(ref e) => fmt.write_fmt(format_args!("Process control: {}", e)),
      ProcessControlRecv(ref e) => fmt.write_fmt(format_args!("Process control: {}", e)),
      PgConnect(ref e) => fmt.write_fmt(format_args!("Connection failed: {}", e)),
      PgErr(ref e)     => fmt.write_fmt(format_args!("Postgres error: {}", e)),
      PgDbErr(ref e)   => fmt.write_fmt(format_args!("Deductive DB (postgres) error: {}", e)),
      PgConnectStr(ref e) => fmt.write_fmt(format_args!("Connection string failed to parse: {}", e)),
      IOErr(ref e) => fmt.write_fmt(format_args!("IO failed: {}", e))
    }
  }
}

impl Debug for ControlError  {
  fn fmt(&self, fmt : &mut Formatter) -> Result<(),::std::fmt::Error> {
    Display::fmt(self, fmt)
  }
}

impl Error for ControlError {
  fn description(&self) -> &str {
    match self {
      &NoDB              => "No database specified",
      &PgConnect(ref e)  => e.description(),
      &PgErr(ref e)      => e.description(),
      &PgDbErr(ref e)    => e.description(),
      &ProcessControlSend(ref e) => e.description(),
      &ProcessControlRecv(ref e) => e.description(),
      &IOErr(ref e)      => e.description(),
      &PgConnectStr(_)   => "Connection string failed to parse"
    }
  }
}

impl From<::postgres::error::ConnectError> for ControlError {
  fn from(ce : ::postgres::error::ConnectError) -> ControlError {PgConnect(ce)}
}

impl From<::postgres::error::Error> for ControlError {
  fn from(e : ::postgres::error::Error) -> ControlError {PgErr(e)}
}

impl From<pg_db::DBError> for ControlError {
  fn from(e : pg_db::DBError) -> ControlError {PgDbErr(e)}
}

impl From<RecvError> for ControlError {
  fn from(e : RecvError) -> ControlError {ProcessControlRecv(e)}
}

impl From<SendError<Command>> for ControlError {
  fn from(e : SendError<Command>) -> ControlError {ProcessControlSend(e)}
}

impl From<::std::io::Error> for ControlError {
  fn from(e : ::std::io::Error) -> ControlError {IOErr(e)}
}

impl<'a> DB {
  fn destroy(&self) -> Result<(), ControlError> {
    match self {
      &DB::Postgres(ref str) => {
        let mut params = try!(str.into_connect_params().map_err(PgConnectStr));
        let old_db = try!(params.database.ok_or(NoDB));
        params.database = Some("postgres".to_owned());
        let conn = try!(Connection::connect(params, SslMode::None));
        let disco_query = format!("SELECT pg_terminate_backend(pg_stat_activity.pid) FROM pg_stat_activity WHERE pg_stat_activity.datname = '{}' AND pid <> pg_backend_pid()", &old_db);
        try!(conn.execute(&disco_query, &[]));
        let drop_query = format!("DROP DATABASE {}", &old_db);
        try!(conn.execute(&drop_query, &[]));
      }
    }
    Ok(())
  }
  fn create(&self) -> Result<(), ControlError> {
    match self {
      &DB::Postgres(ref str) => {
        let mut params = try!(str.into_connect_params().map_err(PgConnectStr));
        let old_db = try!(params.database.ok_or(NoDB));
        params.database = Some("postgres".to_owned());
        let conn = try!(Connection::connect(params, SslMode::None));
        let create_query = format!("CREATE DATABASE {}", &old_db);
        let _ = conn.execute(&create_query, &[]);
      }
    }
    Ok(())
  }
}

pub struct Server<'a> {
  addr : &'a str,
  db : DB,
  thread  : Option<JoinHandle<()>>,
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
  pub fn boot(&mut self) -> Result<(), ControlError> {
    try!(self.db.create());
    let (rpc_server, control, status) = try!(RpcServer::new(self.addr));
    let db = match self.db {
      DB::Postgres(ref s) => {try!(PgDB::new(&s))}
    };
    let holmes = Box::new(holmes::ServerDispatch {
      server : Box::new(HolmesImpl::new(Box::new(db)))
      });
    self.control = Some(control);
    self.status  = Some(status);
    self.thread = Some(rpc_server.serve(holmes));
    Ok(())
  }
  pub fn join(&mut self) {
    let thread = self.thread.take();
    thread.expect("Tried to join non-running server").join().unwrap()
  }
  pub fn shutdown(&mut self) -> Result<(), ControlError> {
    try!(self.control
             .take()
             .expect("No control channel held.")
             .send(Command::Shutdown));
    try!(TcpStream::connect(self.addr));
    let s = try!(self.status
                     .take()
                     .expect("No status channel held.")
                     .recv());
    assert_eq!(s, Status::Offline);
    self.join();
    Ok(())
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
