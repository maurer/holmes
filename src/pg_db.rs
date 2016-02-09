use native_types::*;
use db_types::{types, RowIter};
use db_types::values::Value;
use db_types::types::Type;

use std::convert::From;
use std::fmt::{Formatter};
use std::fmt;

use postgres::{Connection, SslMode};
use postgres as pg;

use std::collections::HashSet;
use std::collections::hash_map::{HashMap};
use std::collections::hash_map::Entry::{Occupied, Vacant};

use postgres::types::ToSql;
use std::sync::Arc;

type ClauseId = (String, i32);
type WhereId = i32;

#[derive(Debug)]
pub enum Error {
  Connect(pg::error::ConnectError),
  Db(pg::error::Error),
  Type(String),
  Internal(String),
  Arg(String)
}

impl From<pg::error::Error> for Error {
  fn from(x : pg::error::Error) -> Error {Error::Db(x)}
}
impl From<pg::error::ConnectError> for Error {
  fn from(x : pg::error::ConnectError) -> Error {Error::Connect(x)}
}

impl ::std::fmt::Display for Error {
  fn fmt (&self, fmt : &mut Formatter) -> fmt::Result {
    match *self {
      Error::Connect(ref x) => x.fmt(fmt),
      Error::Db(ref x) => x.fmt(fmt),
      Error::Type(ref s) =>
        fmt.write_str(&format!("Could not parse db type: {}",
                               s.clone())),
      Error::Internal(ref s) =>
        fmt.write_str(&format!("PgDB Internal Error: {}",
                               s.clone())),
      Error::Arg(ref s) =>
        fmt.write_str(&format!("Bad argument: {}", s))
    }
  }
}

impl ::std::error::Error for Error {
  fn description(&self) -> &str {
    match *self {
      Error::Connect(ref x) => x.description(),
      Error::Db(ref x) => x.description(),
      Error::Type(_) => "Could not parse db types",
      Error::Internal(_) => "PgDB Internal Error",
      Error::Arg(_) => "Bad argument: {}"
    }
  }
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct PgDB {
  conn              : Connection,
  pred_by_name      : HashMap<String, Predicate>,
  insert_by_name    : HashMap<String, String>,
  rule_by_pred_name : HashMap<String, Vec<Arc<Rule>>>,
  rule_exec_cache   : HashMap<Rule, HashSet<Vec<Arc<Value>>>>,
  named_types       : HashMap<String, Arc<Type>>
}

impl PgDB {
  pub fn new(conn_str : &str) -> Result<PgDB> {
    let conn = try!(Connection::connect(conn_str, SslMode::None));

    //Create schemas
    try!(conn.execute("create schema if not exists facts", &[]));

    //Create Tables
    try!(conn.execute("create table if not exists predicates (pred_name varchar not null, ordinal int4 not null, type varchar not null)", &[]));
    try!(conn.execute("create table if not exists rules (id serial, rule varchar not null)", &[]));

    //Create incremental PgDB object
    let mut pg_db = PgDB {
      conn : conn,
      pred_by_name      : HashMap::new(),
      insert_by_name    : HashMap::new(),
      rule_by_pred_name : HashMap::new(),
      rule_exec_cache   : HashMap::new(),
      named_types       : types::default_types().iter().map(|type_| {
                            (type_.name().unwrap().to_owned(), type_.clone())
                          }).collect()
    };

    //Reload predicate cache
    let mut pred_by_name : HashMap<String, Predicate> = HashMap::new();
    {
      let pred_stmt = try!(pg_db.conn.prepare("select pred_name, type from predicates ORDER BY pred_name, ordinal"));
      let pred_types = try!(pred_stmt.query(&[]));
      for type_entry in pred_types.iter() {
        let name : String = type_entry.get(0);
        let h_type_str : String = type_entry.get(1);
        let h_type = match pg_db.get_type(&h_type_str) {
            Some(ty) => ty,
            None => return Err(Error::Type(format!("Type not in registry: {}", h_type_str)))
          };
        match pred_by_name.entry(name.clone()) {
          Vacant(entry) => {
            let mut types = Vec::new();
            types.push(h_type.clone());
            entry.insert(Predicate {
              name  : name.clone(),
              types : types
            });
          }
          Occupied(mut entry) => {
            entry.get_mut().types.push(h_type.clone());
          }
        }
      }
    }

    //Populate fact insert cache
    for pred in pred_by_name.values() {
      &pg_db.gen_insert_stmt(pred);
    }

    //Finish predicate cache
    pg_db.pred_by_name = pred_by_name;

    Ok(pg_db)
  }

  fn gen_insert_stmt(&mut self, pred : &Predicate) {
    let args : Vec<String> = pred.types.iter().enumerate().map(|(k,_)|{
      format!("${}", k + 1)
    }).collect();
    let stmt = format!("insert into facts.{} values ({})",
                       pred.name,
                       args.join(", "));
    self.insert_by_name.insert(pred.name.clone(), stmt);
  }

  fn insert_predicate(&self, pred : &Predicate) -> Result<()> {
    let &Predicate {ref name, ref types} = pred;
    for (ordinal, type_) in types.iter().enumerate() {
      try!(self.conn.execute("insert into predicates \
                              (pred_name, type, ordinal) \
                              values ($1, $2, $3)",
                             &[name,
                               &type_.name().unwrap(),
                               &(ordinal as i32)]));
    }
    let table_str = types.iter().flat_map(|type_| {type_.repr()}).enumerate().map(|(ord, repr)| {format!("arg{} {}", ord, repr)}).collect::<Vec<_>>().join(", ");
    try!(self.conn.execute(&format!("create table facts.{} ({})", name, table_str), &[]));
    Ok(())
  }

  pub fn insert_fact(&mut self, fact : &Fact) -> Result<bool> {
    let stmt : String = try!(self.insert_by_name
      .get(&fact.pred_name)
      .ok_or(Error::Internal("Insert Statement Missing"
                           .to_string()))).clone();
    let argrefs : Vec<&ToSql> = fact.args.iter().flat_map(|x|{x.to_sql().into_iter()}).collect();
    let inserted = try!(self.conn.execute(&stmt, &argrefs)) > 0;
    Ok(inserted)
  }


  pub fn new_rule(&mut self, rule : &Rule) {
    let a_rule : Arc<Rule> = Arc::new(rule.clone());
    for pred in &rule.body {
      match self.rule_by_pred_name.entry(pred.pred_name.clone()) {
        Vacant(entry) => {entry.insert(vec![a_rule.clone()]);}
        Occupied(mut entry) => entry.get_mut().push(a_rule.clone())
      }
    }
  }
  pub fn add_type(&mut self, type_ : Arc<Type>) -> Result<()> {
    let name = type_.name().unwrap();
    if !self.named_types.contains_key(name) {
      self.named_types.insert(name.to_owned(), type_.clone());
      Ok(())
    } else {
      Err(Error::Type(format!("{} already registered", name)))
    }
  }
  pub fn get_type(&self, type_str : &str) -> Option<Arc<Type>> {
    self.named_types.get(type_str).map(|x|{x.clone()})
  }
  pub fn get_rules(&self, pred_name : &str) -> Vec<Arc<Rule>> {
    self.rule_by_pred_name.get(pred_name).map(|x|{x.clone()}).unwrap_or(Vec::new())
  }
  pub fn get_predicate(&self, pred_name : &str) -> Option<&Predicate> {
    self.pred_by_name.get(pred_name)
  }
  pub fn new_predicate(&mut self, pred : &Predicate) -> Result<()> {
    if !valid_name(&pred.name) {
      return Err(Error::Arg("Invalid name: Use lowercase and underscores only".to_string()))
    }
    if self.pred_by_name.contains_key(&pred.name) {
      return Err(Error::Arg(format!("Predicate {} already registered.", &pred.name)))
    }
    try!(self.insert_predicate(&pred));
    self.gen_insert_stmt(&pred);
    self.pred_by_name.insert(pred.name.clone(), pred.clone());
    Ok(())
  }
  pub fn search_facts(&self, query : &Vec<Clause>) -> Result<Vec<Vec<Arc<Value>>>> {

    //Check there is at least one clause
    if query.len() == 0 {
      return Err(Error::Arg("Empty search query".to_string()));
    };

    //Check that clauses:
    // * Have sequential variables
    // * Reference predicates in the database
    // * Only unify variables of equal type
    {
      let mut var_type : Vec<Arc<Type>> = Vec::new();
      for clause in query.iter() {
        let pred = match self.pred_by_name.get(&clause.pred_name) {
          Some(pred) => pred,
          None => return Err(Error::Arg(format!("{} is not a registered predicate.", clause.pred_name))),
        };
        for (idx, slot) in clause.args.iter().enumerate() {
          match *slot {
              MatchExpr::Unbound
            | MatchExpr::Const(_) => (),
              MatchExpr::Var(v) => {
                let v = v as usize;
                if v == var_type.len() {
                  var_type.push(pred.types[idx].clone())
                } else if v > var_type.len() {
                  return Err(Error::Arg(format!("Hole between {} and {} in variable numbering.", var_type.len() - 1, v)))
                } else if var_type[v] != pred.types[idx].clone() {
                  return Err(Error::Arg(format!("Variable {} attempt to unify incompatible types {:?} and {:?}", v, var_type[v], pred.types[idx])))
                }
              }
          }
        }
      }
    }
    
    // Actually build and execute the query
    let mut tables = Vec::new(); //predicate names involved in the query, in sequence
    let mut restricts = Vec::new(); //Unification expressions, indexed by which join they belong on.
    let mut var_names = Vec::new(); //Translation of variable numbers to sql exprs
    let mut var_types = Vec::new(); //Translation of variable numbers to HTypes
    let mut where_clause = Vec::new(); //Constant comparisons
    let mut vals : Vec<&ToSql> = Vec::new(); //Values for passing into the stmt
    for (idxc, clause) in query.iter().enumerate() {
      let table_name = format!("facts.{}", clause.pred_name);
      let alias_name = format!("t{}", idxc);
      let mut clause_elements = Vec::new();
      for (idx, arg) in clause.args.iter().enumerate() {
        match arg {
          &MatchExpr::Unbound => (),
          &MatchExpr::Var(var) => if var >= var_names.len() {
              var_names.push(
                format!("{}.arg{}", alias_name, idx));
              var_types.push(self.pred_by_name[&clause.pred_name].types[idx].clone());
            } else {
              let piece = format!("{}.arg{} = {}", alias_name, idx, var_names[var as usize]);
              if idxc == 0 {
                //For the first element, we have no ON clause, so stick this in WHERE
                where_clause.push(piece);
              } else {
                clause_elements.push(piece);
              }
            },
          &MatchExpr::Const(ref val) => {
            vals.extend(val.to_sql());
            where_clause.push(
              format!("{}.arg{} = ${}", alias_name, idx, vals.len()));
          }
        }
      }
      restricts.push(clause_elements);
      tables.push(format!("{} as {}", table_name, alias_name));
    }
    //Make sure we're never empty on bound variables
    var_names.push("0".to_string());
    let vars = format!("{}", var_names.join(", "));
    tables.reverse();
    restricts.reverse();
    let main_table = tables.pop().unwrap();
    let main_join = restricts.pop();
    assert_eq!(main_join, Some(vec![]));
    let join_blocks : Vec<String> = tables.iter().zip(restricts.iter()).map(|(table, join)| {
        if join.len() == 0 {
          format!("JOIN {} ", table)
        } else {
          format!("JOIN {} ON {}", table, join.join(" AND "))
        }
      }).collect();
    let join_query = join_blocks.join(" ");
    let where_clause = {
      if where_clause.len() == 0 {
        String::new()
      } else {
        format!("WHERE {}", where_clause.join(" AND "))
      }
    };
    let raw_stmt =
      format!("SELECT {} FROM {} {} {}",
              vars, main_table, join_query,
              where_clause);
    let res_stmt = self.conn.prepare(&raw_stmt);
    let stmt = match res_stmt {
      Ok(stmt) => stmt,
      Err(e) => return Err(Error::Internal(
        format!("Preparing statement failed: {}\n{:?}", raw_stmt, e)))
    };
    let res_rows = stmt.query(&vals);
    let rows = match res_rows {
      Ok(rows) => rows,
      Err(e)   => return Err(Error::Internal(
        format!("Executing query failed: {:?}", e)))
    };

    let mut anss : Vec<Vec<Arc<Value>>> = rows.iter().map(|row| {
      let mut row_iter = RowIter::new(&row);
      var_types.iter().map(|type_| {
        type_.extract(&mut row_iter)
      }).collect()
    }).collect();
    anss.dedup();
    Ok(anss)
  }
  pub fn rule_cache_miss(&mut self, rule : &Rule, args : &Vec<Arc<Value>>) -> bool {
    //TODO: persist the cache
    match self.rule_exec_cache.entry(rule.clone()) {
      Vacant(entry) => {
        let mut cache = HashSet::new();
        cache.insert(args.clone());
        entry.insert(cache);
        true
      }
      Occupied(mut entry) => {
        let miss = !entry.get().contains(args);
        entry.get_mut().insert(args.clone());
        miss
      }
    }
  }

}

fn valid_name(name : &String) -> bool {
  name.chars().all( |ch| match ch { 'a'...'z' | '_' => true, _ => false } )
}
