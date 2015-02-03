use native_types::*;
use std::*;

use std::error::FromError;
use std::fmt::{Formatter};
use fact_db::{FactDB, PredResponse};
use holmes_capnp::holmes;
use capnp::list::{struct_list};
use std::str::FromStr;

use postgres::{Connection, ConnectError, Error, SslMode};

use std::collections::hash_map::{HashMap};
use std::collections::hash_map::Entry::{Occupied, Vacant};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DBError {
  ConnectError(ConnectError),
  Error(Error),
  TypeParseError(String)
}
use pg_db::DBError::TypeParseError;

impl FromError<Error> for DBError {
  fn from_error(x : Error) -> DBError {DBError::Error(x)}
}
impl FromError<ConnectError> for DBError {
  fn from_error(x : ConnectError) -> DBError {DBError::ConnectError(x)}
}

impl ::std::fmt::Display for DBError {
  fn fmt (&self, fmt : &mut Formatter) -> fmt::Result {
    match *self {
      DBError::ConnectError(ref x) => x.fmt(fmt),
      DBError::Error(ref x) => x.fmt(fmt),
      DBError::TypeParseError(ref s) =>
        fmt.write_str(format!("Could not parse db type: {}", s.clone()).as_slice())
    }
  }
}

impl ::std::error::Error for DBError {
  fn description(&self) -> &str {
    match *self {
      DBError::ConnectError(ref x) => x.description(),
      DBError::Error(ref x) => x.description(),
      DBError::TypeParseError(_) => "Could not parse db types"
    }
  }
}

impl ::postgres::ToSql for HType {
  fn to_sql(&self, ty: &::postgres::types::Type) -> Result<Option<Vec<u8>>, Error> {
    self.to_string().to_sql(ty)
  }
}

pub struct PgDB {
  conn         : Connection,
  pred_by_name : HashMap<String, Predicate>,
}

impl PgDB {
  pub fn new(conn_str : &str) -> Result<PgDB, DBError> {
    let conn = try!(Connection::connect(conn_str, &SslMode::None));
    try!(conn.execute("create schema if not exists facts", &[]));
    try!(conn.execute("create table if not exists predicates (pred_name varchar not null, ordinal int4 not null, type varchar not null)", &[]));
    let mut pred_by_name : HashMap<String, Predicate> = HashMap::new();
    {
      let pred_stmt = try!(conn.prepare("select pred_name, type from predicates ORDER BY pred_name, ordinal"));
      let pred_types = try!(pred_stmt.query(&[]));
      for type_entry in pred_types {
        let name : String = type_entry.get(0);
        let h_type_str : String = type_entry.get(1);
        let h_type : HType = match FromStr::from_str(h_type_str.as_slice()) {
            Ok(ty) => ty,
            Err(e) => return Err(TypeParseError(e))
          };
        match pred_by_name.entry(name.clone()) {
          Vacant(entry) => {
            let mut types = Vec::new();
            types.push(h_type);
            entry.insert(Predicate {
              name  : name.clone(),
              types : types
            });
          }
          Occupied(mut entry) => {
            entry.get_mut().types.push(h_type);
          }
        }
      }
    }
    Ok(PgDB {
      conn : conn,
      pred_by_name : pred_by_name,
    })
  }

  fn insert_predicate(&self, pred : &Predicate) -> Result<(), DBError> {
    let &Predicate {ref name, ref types} = pred;
    let mut ordinal = 0;
    for h_type in types.iter() {
      try!(self.conn.execute("insert into predicates \
                              (pred_name, type, ordinal) \
                              values ($1, $2, $3)",
                             &[name,
                               h_type,
                               &ordinal]));
      ordinal += 1;
    }
    return Ok(());
  }
}

fn valid_name(name : &String) -> bool {
  name.chars().all( |ch| match ch { 'a'...'z' | '_' => true, _ => false } )
}

impl FactDB for PgDB {
  fn new_predicate<'a>(&mut self, name : &str,
                   types : struct_list::Reader<'a, holmes::h_type::Reader<'a>>)
                   -> PredResponse {
    use fact_db::PredResponse::*;
    let name = String::from_str(name);
    
    if !valid_name(&name) {
      return PredicateInvalid("Invalid name: Use lowercase and underscores only".to_string());
    }
    
    let types = convert_types(types);
    
    if types.len() == 0 {
      return PredicateInvalid("Predicates must have at least one argument.".to_string());
    }
    //Check if we already have a predicate by this name
    match self.pred_by_name.get(&name) {
      Some(p) => {
        if types == p.types {
          //Types match, we're legal
          return PredicateExists;
        } else {
          //Types don't match, throw error
          return PredicateTypeMismatch;
        }
      }
      None => ()
    }
    
    let predicate = Predicate {
      name  : name.clone(),
      types : types
    };
    
    match self.insert_predicate(&predicate) {
      Ok(()) => {}
      Err(e) => {return PredicateInvalid(format!("{:?}", e));}
    }

    self.pred_by_name.insert(name.clone(), predicate);
    PredResponse::PredicateCreated
  }
}
