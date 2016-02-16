//! Holmes
//!
//! Holmes is a Datalog inspired system for binding codependent analyses
//! together.
//!
//! # Tutorial
//!
//! ## Basic Datalog
//! If you are already familiar with logic languages, this section will likely
//! be straightforwards for you, but it may still be useful to provide an
//! overview of basic functions and syntax.
//!
//! Datalog is a forward-chaining logic language. This means that a program
//! written in Datalog consists of a set of rules which "fire" whenever their
//! requirements are met which operate on a database of facts.
//!
//! ### Predicates
//!
//! A predicate represents a property on a list of typed values. For example,
//! to express the distance between two cities in miles, we might write
//!
//! ```
//! # #[macro_use]
//! # extern crate holmes;
//! # use holmes::{Holmes, Result};
//! # use holmes::DB::Memory;
//! # fn f () -> Result<()> {
//! # let mut holmes = try!(Holmes::new(Memory));
//! # let b = &mut holmes;
//! # holmes_exec!(b, {
//! predicate!(distance(string, string, uint64))
//! # });
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! N.B. while this code is being built via doctests, there are a few lines of
//! support code above and below being hidden for clarity. See the complete
//! example at the end of the section for a template.
//!
//! ### Facts
//!
//! Facts are formed by the application of predicates to values. Continuing
//! with the example from before, we can add a fact to the database for the
//! predicate we defined
//!
//! ```
//! # #[macro_use]
//! # extern crate holmes;
//! # use holmes::{Holmes, Result};
//! # use holmes::DB::Memory;
//! # fn f () -> Result<()> {
//! # let mut holmes = try!(Holmes::new(Memory));
//! # let b = &mut holmes;
//! # holmes_exec!(b, {
//! predicate!(distance(string, string, uint64));
//! fact!(distance("New York", "Albuquerque", 1810))
//! # });
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! ### Rules
//!
//! Rules are formed from a body clause and a head clause.
//! When the rule body matches, variable assignments from the match are
//! substituted into the head clause, which is then added to the database.
//! Here, we might want to add the symmetry property to our previous example,
//! e.g. "If the distance from A to B is N, then the distance from B to A is
//! also N".
//!
//! ```
//! # #[macro_use]
//! # extern crate holmes;
//! # use holmes::{Holmes, Result};
//! # use holmes::DB::Memory;
//! # fn f () -> Result<()> {
//! # let mut holmes = try!(Holmes::new(Memory));
//! # let b = &mut holmes;
//! # holmes_exec!(b, {
//! predicate!(distance(string, string, uint64));
//! fact!(distance("New York", "Albuquerque", 1810));
//! rule!(distance(B, A, N) <= distance(A, B, N))
//! # });
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! ### Queries
//!
//! Now that the database has more facts in it than we started with, it makes
//! sense to be able to query the database and see what is inside.
//!
//! ```
//! # #[macro_use]
//! # extern crate holmes;
//! # use holmes::{Holmes, Result};
//! # use holmes::DB::Memory;
//! # use holmes::pg::dyn::values::ToValue;
//! # fn f () -> Result<()> {
//! # let mut holmes_own = try!(Holmes::new(Memory));
//! # let holmes = &mut holmes_own;
//! holmes_exec!(holmes, {
//!   predicate!(distance(string, string, uint64));
//!   fact!(distance("New York", "Albuquerque", 1810));
//!   rule!(distance(B, A, N) <= distance(A, B, N))
//! });
//! let mut res = try!(query!(holmes, distance(A, [_], [_])));
//! # res.sort_by(|x, y| x.partial_cmp(y).unwrap_or(
//! #   ::std::cmp::Ordering::Greater));
//! assert_eq!(res,
//!            vec![vec!["Albuquerque".to_value()],
//!                 vec!["New York".to_value()]]);
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! ### Recursive Rules
//!
//! Let's go one step further, and use a rule to check connectivity between
//! cities, based on the facts in the database. We want to express "If A
//! connects to B, and B connects to C, then A connects to C".
//!
//! ```
//! # #[macro_use]
//! # extern crate holmes;
//! # use holmes::{Holmes, Result};
//! # use holmes::DB::Memory;
//! # use holmes::pg::dyn::values::ToValue;
//! # fn f () -> Result<()> {
//! # let mut holmes_own = try!(Holmes::new(Memory));
//! # let holmes = &mut holmes_own;
//! holmes_exec!(holmes, {
//!   predicate!(distance(string, string, uint64));
//!   fact!(distance("New York", "Albuquerque", 1810));
//!   fact!(distance("New York", "Las Vegas", 2225));
//!   fact!(distance("Las Vegas", "Palo Alto", 542));
//!   fact!(distance("Rome", "Florence", 173));
//!   rule!(distance(B, A, N) <= distance(A, B, N));
//!   predicate!(connected(string, string));
//!   rule!(connected(A, B) <= distance(A, B, [_]));
//!   rule!(connected(A, C) <= connected(A, B) & connected(B, C))
//! });
//! assert_eq!(try!(query!(holmes, connected(("Rome"), ("Las Vegas")))).len(),
//!            0);
//! let mut res = try!(query!(holmes, connected(("Palo Alto"), x)));
//! # res.sort_by(|x, y| x.partial_cmp(y).unwrap_or(
//! #   ::std::cmp::Ordering::Greater));
//! assert_eq!(res,
//!            vec![vec!["Albuquerque".to_value()],
//!                 vec!["Las Vegas".to_value()],
//!                 vec!["New York".to_value()],
//!                 vec!["Palo Alto".to_value()]]);
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! ### Complete Example
//!
//! Finally, just for reference (so you can actually write your own program
//! using this) here's the unredacted version of that last example:
//!
//! ```
//! #[macro_use]
//! extern crate holmes;
//! use holmes::{Holmes, Result};
//! use holmes::DB::Memory;
//! use holmes::pg::dyn::values::ToValue;
//! fn f () -> Result<()> {
//!   // I'm using `Memory` in the examples, but you probably don't want to use
//!   // it in your own code. Check out `Holmes::DB`'s wings to see what your
//!   // options are. `Memory` is super slow for the moment, and I don't forsee
//!   // taking time to optimize it.
//!   let mut holmes_own = try!(Holmes::new(Memory));
//!   // For the moment, the `holmes_exec` macro needs a &mut ident. I'll
//!   // try to make this more flexible in the future.
//!   let holmes = &mut holmes_own;
//!   holmes_exec!(holmes, {
//!     predicate!(distance(string, string, uint64));
//!     fact!(distance("New York", "Albuquerque", 1810));
//!     fact!(distance("New York", "Las Vegas", 2225));
//!     fact!(distance("Las Vegas", "Palo Alto", 542));
//!     fact!(distance("Rome", "Florence", 173));
//!     rule!(distance(B, A, N) <= distance(A, B, N));
//!     predicate!(connected(string, string));
//!     rule!(connected(A, B) <= distance(A, B, [_]));
//!     rule!(connected(A, C) <= connected(A, B) & connected(B, C))
//!   });
//!   assert_eq!(try!(query!(holmes, connected(("Rome"), ("Las Vegas")))).len(),
//!              0);
//!   let mut res = try!(query!(holmes, connected(("Palo Alto"), x)));
//!   // Order is not gauranteed when it comes back from the query, so I
//!   // sort it in the example to get the doctest to pass. `Value` only has
//!   // `PartialOrd` implemented for it, since there isn't a clean comparison
//!   // between `Value`s of different types, so I just default to `Greater`.
//!   res.sort_by(|x, y| x.partial_cmp(y).unwrap_or(
//!     ::std::cmp::Ordering::Greater));
//!   assert_eq!(res,
//!              vec![vec!["Albuquerque".to_value()],
//!                   vec!["Las Vegas".to_value()],
//!                   vec!["New York".to_value()],
//!                   vec!["Palo Alto".to_value()]]);
//!   Ok(())
//! }
//! fn main () {f().unwrap()}
//! ```
//!
// TODO extend crate docs with example and tutorial on how to use functions
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
use mem_db::MemDB;

/// Defines the database connection parameters
#[derive (Clone)]
pub enum DB {
  /// A postgres database, via a connection string
  Postgres(String),
  /// A memory backed database, *VERY INEFFICIENT*
  Memory
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
      &DB::Memory => ()
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
      &DB::Memory => Ok(Box::new(MemDB::new()))
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
