//! Holmes
//!
//! Holmes is a Datalog inspired system for binding codependent analyses
//! together.
// TODO extend crate docs with example and tutorial on how to write a Holmes
// program.
#![warn(missing_docs)]
extern crate postgres;
extern crate postgres_array;
extern crate rustc_serialize;

pub mod pg;
pub mod fact_db;
pub mod mem_db;
pub mod engine;
pub mod edsl;

use pg::dyn::{Value, Type};
use engine::types::*;
use fact_db::FactDB;

/// Defines the database connection parameters
// Right now we just do a postgres connection string, other options
// (in memory? other dbs?)
// would go here eventually for instructions on constructing the Holmes object.
#[derive (Clone)]
pub enum DB {
  /// A postgres database, with a connection string
  Postgres(String)
}

/// Ways that a `Holmes` operation might go wrong
// TODO: refactor based on cause() to have more semantic error types
pub enum Error {
  /// No database was specified
  NoDB,
  /// There was an error connecting to the database
  PgConnect(::postgres::error::ConnectError),
  /// There was a database error when setting up the database
  Pg(::postgres::error::Error),
  /// There was an error in the Holmes fact database layer
  Db(Box<::std::error::Error>),
  /// General IO Error
  IO(::std::io::Error),
  /// Parsing the connection string failed
  PgConnectStr(Box<::std::error::Error + Send + Sync>),
  /// There was an error in the Holmes engine layer
  Engine(engine::Error)
}

use engine::Engine;
use pg::PgDB;

/// `Result` is a shorthand type for the standard `Result` specialized to our
/// `Error type.
pub type Result<T> = ::std::result::Result<T, Error>;

use std::fmt::{Debug, Display, Formatter};

impl Display for Error {
  fn fmt(&self, fmt : &mut Formatter)
        -> ::std::result::Result<(),::std::fmt::Error> {
    use self::Error::*;
    match *self {
      NoDB             => fmt.write_str("No database specified"),
      PgConnect(ref e) => fmt.write_fmt(format_args!("Connection failed: {}", e)),
      Pg(ref e)     => fmt.write_fmt(format_args!("Postgres error: {}", e)),
      Db(ref e)   => fmt.write_fmt(format_args!("Deductive DB error: {}", e)),
      PgConnectStr(ref e) => fmt.write_fmt(format_args!("Connection string failed to parse: {}", e)),
      IO(ref e) => fmt.write_fmt(format_args!("IO failed: {}", e)),
      Engine(ref e) => fmt.write_fmt(format_args!("Engine Error: {}", e))
    }
  }
}

impl Debug for Error  {
  fn fmt(&self, fmt : &mut Formatter)
        -> ::std::result::Result<(),::std::fmt::Error> {
    Display::fmt(self, fmt)
  }
}

impl ::std::error::Error for Error {
  fn description(&self) -> &str {
    use self::Error::*;
    match self {
      &NoDB              => "No database specified",
      &PgConnect(ref e)  => e.description(),
      &Pg(ref e)         => e.description(),
      &Db(ref e)         => e.description(),
      &IO(ref e)         => e.description(),
      &Engine(ref e)     => e.description(),
      &PgConnectStr(_)   => "Connection string failed to parse"
    }
  }
}

impl From<::postgres::error::ConnectError> for Error {
  fn from(ce : ::postgres::error::ConnectError) -> Error {Error::PgConnect(ce)}
}

impl From<::postgres::error::Error> for Error {
  fn from(e : ::postgres::error::Error) -> Error {Error::Pg(e)}
}

impl From<Box<::std::error::Error>> for Error {
  fn from(e : Box<::std::error::Error>) -> Error {Error::Db(e)}
}

impl From<engine::Error> for Error {
  fn from(e : engine::Error) -> Error {Error::Engine(e)}
}

impl From<::std::io::Error> for Error {
  fn from(e : ::std::io::Error) -> Error {Error::IO(e)}
}

impl<'a> DB {
  /// Destroy the fact database entirely
  ///
  /// * Kicks everyone off the database
  /// * Drops the database
  fn destroy(&self) -> Result<()> {
    match self {
      &DB::Postgres(ref str) => {
        use postgres::{Connection, SslMode, IntoConnectParams};
        let mut params = try!(str.into_connect_params()
                                 .map_err(Error::PgConnectStr));
        let old_db = try!(params.database.ok_or(Error::NoDB));
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
  /// Creates a fresh fact database
  fn create(&self) -> Result<Box<FactDB>> {
    match self {
      &DB::Postgres(ref str) => {
        use postgres::{Connection, SslMode, IntoConnectParams};
        let mut params = try!(str.into_connect_params()
                                 .map_err(Error::PgConnectStr));
        let old_db = try!(params.database.ok_or(Error::NoDB));
        params.database = Some("postgres".to_owned());
        let conn = try!(Connection::connect(params, SslMode::None));
        let create_query = format!("CREATE DATABASE {}", &old_db);
        //TODO accept success or db already exists, not other errors
        let _ = conn.execute(&create_query, &[]);
        Ok(Box::new(try!(PgDB::new(str))))
      }
    }
  }
}

/// Encapsulates the user-level interface to a holmes database
pub struct Holmes {
  db : DB,
  engine : Engine
}

impl Holmes {
  /// Create a new holmes instance from a db specification
  pub fn new(db : DB) -> Result<Holmes> {
    Ok(Holmes {
      engine : Engine::new(try!(db.create())),
      db : db
    })
  }

  /// Tear down and destroy the database.
  /// THIS DELETES YOUR DATA
  pub fn destroy(self) -> Result<()> {
    let db = self.db.clone();
    drop(self);
    db.destroy()
  }

  /// Register a new predicate with the database. This should be done before
  /// adding facts to that predicate or rules that refer to it.
  /// Do not attempt to register two predicates with the same name.
  pub fn add_predicate(&mut self, pred : &engine::types::Predicate)
                      -> Result<()> {
    self.engine.new_predicate(pred).map_err(|e| {Error::Engine(e)})
  }

  /// Add a new fact to the database. The predicate must be registered first.
  /// Duplicate facts are ignored.
  pub fn add_fact(&mut self, fact : &engine::types::Fact)
                      -> Result<()> {
    self.engine.new_fact(fact).map_err(|e| {Error::Engine(e)})
  }

  /// Query the database with the right hand side of a datalog rule, returning
  /// all legal assignments to the variables to make it true
  pub fn query(&mut self, query : &Vec<engine::types::Clause>) -> Result<Vec<Vec<Value>>> {
    self.engine.derive(query).map_err(|e| {Error::Engine(e)})
  }

  /// Get a dynamic type registered with Holmes by name. If it is present, it
  /// returns `Some(Type)`, otherwise it returns `None`. The type database is
  /// prepopulated with a few defaults, but otherwise this will only work if
  /// you registered the type via `add_type`.
  pub fn get_type(&self, name : &str) -> Option<Type> {
    self.engine.get_type(name)
  }

  /// Registers a new type with Holmes. It must be a named type, e.g.
  /// `type.name()` would return the `Some` branch.
  /// Types should be registered exactly once
  pub fn add_type(&mut self, type_ : Type) -> Result<()> {
    self.engine.add_type(type_).map_err(|e| {Error::Engine(e)})
  }

  /// Adds a new inference rule to the Holmes program
  pub fn add_rule(&mut self, rule : &engine::types::Rule) -> Result<()> {
    self.engine.new_rule(rule).map_err(|e| {Error::Engine(e)})
  }

  /// Registers a dynamically typed function for use in Holmes where clauses
  pub fn reg_func(&mut self,
                  name : String,
                  input_type  : Type,
                  output_type : Type,
                  func : Box<Fn(Value) -> Value>) -> Result<()> {
    self.engine.reg_func(name, engine::types::Func {
      input_type  : input_type,
      output_type : output_type,
      run : func
    });
    Ok(())
  }
}
