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
pub mod engine;

use pg::dyn::{Value, Type};
use engine::types::*;

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
  PgDb(pg::Error),
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
      PgDb(ref e)   => fmt.write_fmt(format_args!("Deductive DB (postgres) error: {}", e)),
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
      &PgDb(ref e)       => e.description(),
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

impl From<pg::Error> for Error {
  fn from(e : pg::Error) -> Error {Error::PgDb(e)}
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
  fn create(&self) -> Result<PgDB> {
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
        Ok(try!(PgDB::new(str)))
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

#[macro_export]
macro_rules! htype {
  ($holmes:ident, [$t:tt]) => { ::holmes::pg::dyn::types::List::new(htype!($holmes, $t)) };
  ($holmes:ident, ($($t:tt),*)) => { ::holmes::pg::dyn::types::Tuple::new(vec![$(htype!($holmes, $t)),*]) };
  ($holmes:ident, $i:ident) => { $holmes.get_type(stringify!($i)).unwrap() };
}

#[macro_export]
macro_rules! holmes_exec {
  ($holmes:ident, { $( $action:expr );* }) => {
      {
        $( try!($action($holmes)); );*
        let res : ::holmes::Result<()> = Ok(());
        res
      }
  };
}

#[macro_export]
macro_rules! predicate {
  ($holmes:ident, $pred_name:ident($($t:tt),*)) => {{
    let types = vec![$(htype!($holmes, $t),)*];
    $holmes.add_predicate(&::holmes::engine::types::Predicate {
      name  : stringify!($pred_name).to_string(),
      types : types
    })
  }};
  ($pred_name:ident($($t:tt),*)) => { |holmes : &mut Holmes| {
    let res : ::holmes::Result<()> = predicate!(holmes, $pred_name($($t),*));
    res
  }};
}

#[macro_export]
macro_rules! fact {
  ($holmes:ident, $pred_name:ident($($a:expr),*)) => {
    $holmes.add_fact(&::holmes::engine::types::Fact {
      pred_name : stringify!($pred_name).to_string(),
      args : vec![$(::holmes::pg::dyn::values::ToValue::to_value($a)),*]
    })
  };
  ($pred_name:ident($($a:expr),*)) => { |holmes : &mut Holmes| {
    let res : ::holmes::Result<()> = fact!(holmes, $pred_name($($a),*));
    res
  }};
}

#[macro_export]
macro_rules! bind_match {
  ($vars:ident, $n:ident, [ $bm:tt ]) => { ::holmes::engine::types::BindExpr::Iterate(Box::new(bind_match!($vars, $n, $bm))) };
  ($vars:ident, $n:ident, {$($bm:tt),*}) => {
    ::holmes::engine::types::BindExpr::Destructure(vec![$(bind_match!($vars, $n, $bm)),*])
  };
  ($vars:ident, $n:ident, $cm:tt) => { ::holmes::engine::types::BindExpr::Normal(clause_match!($vars, $n, $cm)) };
}

#[macro_export]
macro_rules! clause_match {
  ($vars:ident, $n:ident, [_]) => { ::holmes::engine::types::MatchExpr::Unbound };
  ($vars:ident, $n:ident, ($v:expr)) => {
      ::holmes::engine::types::MatchExpr::Const(::holmes::pg::dyn::values::ToValue::to_value($v)) };
  ($vars:ident, $n:ident, $m:ident) => {{
    use std::collections::hash_map::Entry::*;
    use ::holmes::engine::types::MatchExpr::*;
    match $vars.entry(stringify!($m).to_string()) {
      Occupied(entry) => Var(*entry.get()),
      Vacant(entry) => {
        $n = $n + 1;
        entry.insert($n - 1);
        Var($n - 1)
      }
    }
  }};
}

#[macro_export]
macro_rules! query {
  ($holmes:ident, $($pred_name:ident($($m:tt),*))&*) => {{
    use std::collections::HashMap;
    let mut vars : HashMap<String, ::holmes::engine::types::Var> = HashMap::new();
    let mut n : ::holmes::engine::types::Var = 0;
    $holmes.query(&vec![$(::holmes::engine::types::Clause {
      pred_name : stringify!($pred_name).to_string(),
      args : vec![$(clause_match!(vars, n, $m)),*]
    }),*])
  }}
}

/// Helper function for macros. Not intended for direct use.
/// Turns `MatchExpr::Var` into `Expr::Var`
pub fn var_to_evar(var : MatchExpr) -> Expr {
  match var {
    MatchExpr::Var(var_no) => Expr::Var(var_no),
    x => panic!("var_to_evar was passed nonvar: {:?}", x)
  }
}

#[macro_export]
macro_rules! hexpr {
  ($vars:ident, $n:ident, [$hexpr_name:ident]) => {
    ::holmes::var_to_evar(clause_match!($vars, $n, $hexpr_name))
  };
  ($vars:ident, $n:ident, ($hexpr:expr)) => {
    ::holmes::engine::types::Expr::Val(::holmes::pg::dyn::values::ToValue::to_value($hexpr))
  };
  ($vars:ident, $n:ident, {$hexpr_func:ident($($hexpr_arg:tt),*)}) => {
    ::holmes::engine::types::Expr::App(stringify!($hexpr_func).to_string(), vec![$(hexpr!($vars, $n, $hexpr_arg)),*])
  };
}

#[macro_export]
macro_rules! rule {
  ($holmes:ident, $head_name:ident($($m:tt),*) <= $($body_name:ident($($mb:tt),*))&*,
   {$(let $bind:tt = $hexpr:tt);*}) => {{
    use std::collections::HashMap;
    let mut vars : HashMap<String, ::holmes::engine::types::Var> = HashMap::new();
    let mut n : ::holmes::engine::types::Var = 0;
    $holmes.add_rule(&::holmes::engine::types::Rule {
      body : vec![$(::holmes::engine::types::Clause {
        pred_name : stringify!($body_name).to_string(),
        args : vec![$(clause_match!(vars, n, $mb)),*]
      }),*],
      head : ::holmes::engine::types::Clause {
        pred_name : stringify!($head_name).to_string(),
        args : vec![$(clause_match!(vars, n, $m)),*]
      },
      wheres : vec! [$(::holmes::engine::types::WhereClause {
        lhs   : bind_match!(vars, n, $bind),
        rhs   : hexpr!(vars, n, $hexpr)
      }),*]
    })
  }};
  ($holmes:ident, $head_name:ident($($m:tt),*) <= $($body_name:ident($($mb:tt),*))&*,
   {$(let $($bind:tt),* = $hexpr:tt);*}) => {{
    use std::collections::HashMap;
    let mut vars : HashMap<String, ::holmes::engine::types::Var> = HashMap::new();
    let mut n : ::holmes::engine::types::Var = 0;
    $holmes.add_rule(&::holmes::engine::types::Rule {
      body : vec![$(::holmes::engine::types::Clause {
        pred_name : stringify!($body_name).to_string(),
        args : vec![$(clause_match!(vars, n, $mb)),*]
      }),*],
      head : ::holmes::engine::types::Clause {
        pred_name : stringify!($head_name).to_string(),
        args : vec![$(clause_match!(vars, n, $m)),*]
      },
      wheres : vec! [$(::holmes::engine::types::WhereClause {
        lhs   : ::holmes::engine::types::BindExpr::Destructure(
                  vec![$(bind_match!(vars, n, $bind)),*]),
        rhs   : hexpr!(vars, n, $hexpr)
      }),*]
    })
  }};
  ($($head_name:ident($($m:tt),*)),* <= $($body_name:ident($($mb:tt),*))&*) => {
    |holmes : &mut Holmes| {
      rule!(holmes, $($head_name($($m),*)),* <= $($body_name($($mb),*))&*, {})
    }
  };
  ($($head_name:ident($($m:tt),*)),* <= $($body_name:ident($($mb:tt),*))&*, {$(let $($bind:tt),* = $hexpr:tt);*}) => {
    |holmes : &mut Holmes| {
      rule!(holmes, $($head_name($($m),*)),* <= $($body_name($($mb),*))&*, {$(let $($bind),* = $hexpr);*})
    }
  };

}

#[macro_export]
macro_rules! func {
  ($holmes:ident, let $name:ident : $src:tt -> $dst:tt = $body:expr) => {{
    let src = htype!($holmes, $src);
    let dst = htype!($holmes, $dst);
    $holmes.reg_func(stringify!($name).to_string(),
                     src, dst,
                     Box::new($body))
  }};
  (let $name:ident : $src:tt -> $dst:tt = $body:expr) => {
    |holmes : &mut Holmes| {
      func!(holmes, let $name : $src -> $dst = $body)
    }
  };
}
