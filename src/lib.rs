extern crate postgres;
extern crate postgres_array;
extern crate rustc_serialize;

pub mod fact_db;
pub mod pg_db;
pub mod engine;

pub mod native_types;

#[derive (Clone)]
pub enum DB {
  Postgres(String)
}

pub enum Error {
  NoDB,
  PgConnect(::postgres::error::ConnectError),
  PgErr(::postgres::error::Error),
  PgDbErr(pg_db::DBError),
  IOErr(::std::io::Error),
  PgConnectStr(Box<::std::error::Error + Send + Sync>),
  EngineErr(engine::Error)
}

use self::Error::*;

use engine::Engine;
use pg_db::PgDB;
use fact_db::FactDB;
use native_types::*;

pub type Result<T> = ::std::result::Result<T, Error>;

use std::fmt::{Debug, Display, Formatter};

impl Display for Error {
  fn fmt(&self, fmt : &mut Formatter)
        -> ::std::result::Result<(),::std::fmt::Error> {
    match *self {
      NoDB             => fmt.write_str("No database specified"),
      PgConnect(ref e) => fmt.write_fmt(format_args!("Connection failed: {}", e)),
      PgErr(ref e)     => fmt.write_fmt(format_args!("Postgres error: {}", e)),
      PgDbErr(ref e)   => fmt.write_fmt(format_args!("Deductive DB (postgres) error: {}", e)),
      PgConnectStr(ref e) => fmt.write_fmt(format_args!("Connection string failed to parse: {}", e)),
      IOErr(ref e) => fmt.write_fmt(format_args!("IO failed: {}", e)),
      EngineErr(ref e) => fmt.write_fmt(format_args!("Engine Error: {}", e))
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
    match self {
      &NoDB              => "No database specified",
      &PgConnect(ref e)  => e.description(),
      &PgErr(ref e)      => e.description(),
      &PgDbErr(ref e)    => e.description(),
      &IOErr(ref e)      => e.description(),
      &EngineErr(ref e)  => e.description(),
      &PgConnectStr(_)   => "Connection string failed to parse"
    }
  }
}

impl From<::postgres::error::ConnectError> for Error {
  fn from(ce : ::postgres::error::ConnectError) -> Error {PgConnect(ce)}
}

impl From<::postgres::error::Error> for Error {
  fn from(e : ::postgres::error::Error) -> Error {PgErr(e)}
}

impl From<pg_db::DBError> for Error {
  fn from(e : pg_db::DBError) -> Error {PgDbErr(e)}
}

impl From<engine::Error> for Error {
  fn from(e : engine::Error) -> Error {EngineErr(e)}
}

impl From<::std::io::Error> for Error {
  fn from(e : ::std::io::Error) -> Error {IOErr(e)}
}

impl<'a> DB {
  fn destroy(&self) -> Result<()> {
    match self {
      &DB::Postgres(ref str) => {
        use postgres::{Connection, SslMode, IntoConnectParams};
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
  fn create(&self) -> Result<Box<FactDB>> {
    match self {
      &DB::Postgres(ref str) => {
        use postgres::{Connection, SslMode, IntoConnectParams};
        let mut params = try!(str.into_connect_params().map_err(PgConnectStr));
        let old_db = try!(params.database.ok_or(NoDB));
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

pub struct Holmes {
  db : DB,
  engine : Engine
}

impl Holmes {
  pub fn new(db : DB) -> Result<Holmes> {
    Ok(Holmes {
      engine : Engine::new(try!(db.create())),
      db : db
    })
  }
  pub fn destroy(self) -> Result<()> {
    let db = self.db.clone();
    drop(self);
    db.destroy()
  }
  pub fn add_predicate(&mut self, pred : &native_types::Predicate)
                      -> Result<()> {
    self.engine.new_predicate(pred).map_err(|e| {EngineErr(e)})
  }
  pub fn add_fact(&mut self, fact : &native_types::Fact)
                      -> Result<()> {
    self.engine.new_fact(fact).map_err(|e| {EngineErr(e)})
  }
  pub fn query(&mut self, query : &Vec<native_types::Clause>) -> Result<Vec<Vec<native_types::HValue>>> {
    self.engine.derive(query).map_err(|e| {EngineErr(e)})
  }
  pub fn add_rule(&mut self, rule : &native_types::Rule) -> Result<()> {
    self.engine.new_rule(rule).map_err(|e| {EngineErr(e)})
  }
  pub fn reg_func(&mut self,
                  name : String,
                  input_type  : native_types::HType,
                  output_type : native_types::HType,
                  func : Box<Fn(native_types::HValue) -> native_types::HValue>) -> Result<()> {
    self.engine.reg_func(name, native_types::HFunc {
      input_type  : input_type,
      output_type : output_type,
      run : func
    });
    Ok(())
  }
}

#[macro_export]
macro_rules! htype_raw {
  (string) => { ::holmes::native_types::HType::HString};
  (blob  ) => { ::holmes::native_types::HType::Blob};
  (uint64) => { ::holmes::native_types::HType::UInt64};
}

#[macro_export]
macro_rules! htype {
  ([$t:tt]) => { ::holmes::native_types::HType::List(Box::new(htype!($t))) };
  (($($t:tt),*)) => { ::holmes::native_types::HType::Tuple(vec![$(htype!($t)),*]) };
  ($i:ident) => { htype_raw!($i) };
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
  ($holmes:ident, $pred_name:ident($($t:tt),*)) => {
    $holmes.add_predicate(&::holmes::native_types::Predicate {
      name  : stringify!($pred_name).to_string(),
      types : vec![$(htype!($t),)*]
    })
  };
  ($pred_name:ident($($t:tt),*)) => { |holmes : &mut Holmes| {
    let res : ::holmes::Result<()> = predicate!(holmes, $pred_name($($t),*));
    res
  }};
}

#[macro_export]
macro_rules! fact {
  ($holmes:ident, $pred_name:ident($($a:expr),*)) => {
    $holmes.add_fact(&::holmes::native_types::Fact {
      pred_name : stringify!($pred_name).to_string(),
      args : vec![$(::holmes::native_types::ToHValue::to_hvalue($a)),*]
    })
  };
  ($pred_name:ident($($a:expr),*)) => { |holmes : &mut Holmes| {
    let res : ::holmes::Result<()> = fact!(holmes, $pred_name($($a),*));
    res
  }};
}

#[macro_export]
macro_rules! bind_match {
  ($vars:ident, $n:ident, [ $bm:tt ]) => { ::holmes::native_types::BindExpr::Iterate(Box::new(bind_match!($vars, $n, $bm))) };
  ($vars:ident, $n:ident, {$($bm:tt),*}) => {
    ::holmes::native_types::BindExpr::Destructure(vec![$(bind_match!($vars, $n, $bm)),*])
  };
  ($vars:ident, $n:ident, $cm:tt) => { ::holmes::native_types::BindExpr::Normal(clause_match!($vars, $n, $cm)) };
}

#[macro_export]
macro_rules! clause_match {
  ($vars:ident, $n:ident, [_]) => { ::holmes::native_types::MatchExpr::Unbound };
  ($vars:ident, $n:ident, ($v:expr)) => {
      ::holmes::native_types::MatchExpr::HConst(::holmes::native_types::ToHValue::to_hvalue($v)) };
  ($vars:ident, $n:ident, $m:ident) => {{
    use std::collections::hash_map::Entry::*;
    use ::holmes::native_types::MatchExpr::*;
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
    let mut vars : HashMap<String, ::holmes::native_types::HVar> = HashMap::new();
    let mut n : ::holmes::native_types::HVar = 0;
    $holmes.query(&vec![$(::holmes::native_types::Clause {
      pred_name : stringify!($pred_name).to_string(),
      args : vec![$(clause_match!(vars, n, $m)),*]
    }),*])
  }}
}

pub fn var_to_evar(var : MatchExpr) -> Expr {
  match var {
    MatchExpr::Var(var_no) => Expr::EVar(var_no),
    x => panic!("var_to_evar was passed nonvar: {:?}", x)
  }
}

#[macro_export]
macro_rules! hexpr {
  ($vars:ident, $n:ident, [$hexpr_name:ident]) => {
    ::holmes::var_to_evar(clause_match!($vars, $n, $hexpr_name))
  };
  ($vars:ident, $n:ident, ($hexpr:expr)) => {
    ::holmes::native_types::Expr::EVal(::holmes::native_types::ToHValue::to_hvalue($hexpr))
  };
  ($vars:ident, $n:ident, {$hexpr_func:ident($($hexpr_arg:tt),*)}) => {
    ::holmes::native_types::Expr::EApp(stringify!($hexpr_func).to_string(), vec![$(hexpr!($vars, $n, $hexpr_arg)),*])
  };
}

#[macro_export]
macro_rules! rule {
  ($holmes:ident, $head_name:ident($($m:tt),*) <= $($body_name:ident($($mb:tt),*))&*,
   {$(let $bind:tt = $hexpr:tt);*}) => {{
    use std::collections::HashMap;
    let mut vars : HashMap<String, ::holmes::native_types::HVar> = HashMap::new();
    let mut n : ::holmes::native_types::HVar = 0;
    $holmes.add_rule(&::holmes::native_types::Rule {
      body : vec![$(::holmes::native_types::Clause {
        pred_name : stringify!($body_name).to_string(),
        args : vec![$(clause_match!(vars, n, $mb)),*]
      }),*],
      head : ::holmes::native_types::Clause {
        pred_name : stringify!($head_name).to_string(),
        args : vec![$(clause_match!(vars, n, $m)),*]
      },
      wheres : vec! [$(::holmes::native_types::WhereClause {
        lhs   : bind_match!(vars, n, $bind),
        rhs   : hexpr!(vars, n, $hexpr)
      }),*]
    })
  }};
  ($holmes:ident, $head_name:ident($($m:tt),*) <= $($body_name:ident($($mb:tt),*))&*,
   {$(let $($bind:tt),* = $hexpr:tt);*}) => {{
    use std::collections::HashMap;
    let mut vars : HashMap<String, ::holmes::native_types::HVar> = HashMap::new();
    let mut n : ::holmes::native_types::HVar = 0;
    $holmes.add_rule(&::holmes::native_types::Rule {
      body : vec![$(::holmes::native_types::Clause {
        pred_name : stringify!($body_name).to_string(),
        args : vec![$(clause_match!(vars, n, $mb)),*]
      }),*],
      head : ::holmes::native_types::Clause {
        pred_name : stringify!($head_name).to_string(),
        args : vec![$(clause_match!(vars, n, $m)),*]
      },
      wheres : vec! [$(::holmes::native_types::WhereClause {
        lhs   : ::holmes::native_types::BindExpr::Destructure(
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
  ($holmes:ident, let $name:ident : $src:tt -> $dst:tt = $body:expr) => {
    $holmes.reg_func(stringify!($name).to_string(),
                     htype!($src),
                     htype!($dst),
                     Box::new($body))
  };
  (let $name:ident : $src:tt -> $dst:tt = $body:expr) => {
    |holmes : &mut Holmes| {
      func!(holmes, let $name : $src -> $dst = $body)
    }
  };
}
