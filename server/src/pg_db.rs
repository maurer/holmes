use native_types::*;
use native_types::HType::*;
use std::*;

use std::error::FromError;
use std::fmt::{Formatter};
use fact_db::*;
use std::str::FromStr;

use postgres::{Connection, ConnectError, Error, SslMode};

use std::collections::hash_map::{HashMap};
use std::collections::hash_map::Entry::{Occupied, Vacant};

use postgres::ToSql;
use std::iter::IteratorExt;
use std::slice::SliceConcatExt;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DBError {
  ConnectError(ConnectError),
  Error(Error),
  TypeParseError(String),
  InternalError(String)
}
use pg_db::DBError::TypeParseError;
use pg_db::DBError::InternalError;

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
        fmt.write_str(format!("Could not parse db type: {}",
                              s.clone()).as_slice()),
      DBError::InternalError(ref s) =>
        fmt.write_str(format!("PgDB Internal Error: {}",
                              s.clone()).as_slice())
    }
  }
}

impl ::std::error::Error for DBError {
  fn description(&self) -> &str {
    match *self {
      DBError::ConnectError(ref x) => x.description(),
      DBError::Error(ref x) => x.description(),
      DBError::TypeParseError(_) => "Could not parse db types",
      DBError::InternalError(_) => "PgDB Internal Error"
    }
  }
}

impl ToSql for HType {
  fn to_sql(&self, ty: &::postgres::types::Type) -> Result<Option<Vec<u8>>, Error> {
    self.to_string().to_sql(ty)
  }
}

impl<'a> ToSql for HValue<'a> {
  fn to_sql(&self, ty: &::postgres::types::Type) -> Result<Option<Vec<u8>>, Error> {
    use native_types::HValue::*;
    match self {
      &UInt64V(i)  => (i as i64).to_sql(ty),
      &HStringV(ref s) => s.clone().to_sql(ty),
      &BlobV(b)    => b.to_sql(ty),
    }
  }
}

pub struct PgDB {
  conn           : Connection,
  pred_by_name   : HashMap<String, Predicate>,
  insert_by_name : HashMap<String, String>
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
    let mut pg_db = PgDB {
      conn : conn,
      pred_by_name : HashMap::new(),
      insert_by_name : HashMap::new()
    };
    for pred in pred_by_name.values() {
      &pg_db.gen_insert_stmt(pred);
    }
    pg_db.pred_by_name = pred_by_name;
    Ok(pg_db)
  }

  fn gen_insert_stmt(&mut self, pred : &Predicate) {
    let args : Vec<String> = pred.types.iter().enumerate().map(|(k,_)|{
      format!("${}", k + 1)
    }).collect();
    let stmt = format!("insert into facts.{} values ({})",
                       pred.name,
                       args.connect(", "));
    self.insert_by_name.insert(pred.name.clone(), stmt);
  }

  fn insert_predicate(&self, pred : &Predicate) -> Result<(), DBError> {
    let &Predicate {ref name, ref types} = pred;
    let mut table_str = "(".to_string();
    for (ordinal, h_type) in types.iter().enumerate() {
      try!(self.conn.execute("insert into predicates \
                              (pred_name, type, ordinal) \
                              values ($1, $2, $3)",
                             &[name,
                               h_type,
                               &(ordinal as i32)]));

      table_str.push_str(format!("arg{} {},", ordinal, h_type_to_sql_type(h_type)).as_slice());
    }
    table_str.pop();
    table_str.push(')');
    try!(self.conn.execute(format!("create table facts.{} {}", name, table_str).as_slice(), &[]));
    return Ok(());
  }

  fn insert_fact(&self, fact : &Fact) -> Result<bool, DBError> {
    let ref stmt : &String = try!(self.insert_by_name
      .get(&fact.pred_name)
      .ok_or(InternalError("Insert Statement Missing"
                           .to_string())));
    let argrefs : Vec<&ToSql> = fact.args.iter().map(|x|{x as &ToSql}).collect();
    Ok(try!(self.conn.execute(stmt, argrefs.as_slice())) > 0)
  }
}

fn valid_name(name : &String) -> bool {
  name.chars().all( |ch| match ch { 'a'...'z' | '_' => true, _ => false } )
}

fn h_type_to_sql_type(h_type : &HType) -> String {
  match h_type {
    &HString => "varchar".to_string(),
    &Blob    => "bytea".to_string(),
    &UInt64  => "int8".to_string(),
  }
}

impl FactDB for PgDB {
  fn new_predicate(&mut self, pred : Predicate) -> PredResponse {
    use fact_db::PredResponse::*;
    if !valid_name(&pred.name) {
      return PredicateInvalid("Invalid name: Use lowercase and underscores only".to_string());
    }
    
    if pred.types.len() == 0 {
      return PredicateInvalid("Predicates must have at least one argument.".to_string());
    }
    //Check if we already have a predicate by this name
    match self.pred_by_name.get(&pred.name) {
      Some(p) => {
        if pred.types == p.types {
          //Types match, we're legal
          return PredicateExists;
        } else {
          //Types don't match, throw error
          return PredicateTypeMismatch;
        }
      }
      None => ()
    }
    
    match self.insert_predicate(&pred) {
      Ok(()) => {}
      Err(e) => {return PredFail(format!("{:?}", e));}
    }
    
    self.gen_insert_stmt(&pred);
    self.pred_by_name.insert(pred.name.clone(), pred);
    PredResponse::PredicateCreated
  }

  fn new_fact(&mut self, fact : &Fact) -> FactResponse {
    use fact_db::FactResponse::*;
    match self.pred_by_name.get(&fact.pred_name) {
      Some(ref pred) => {
        if !fact.args.iter().zip(pred.types.iter()).all(type_check) {
          return FactTypeMismatch;
        }
      }
      None => return FactPredUnreg(fact.pred_name.to_string())
    }
    // We know about the predicate, and the types match, so
    // attempting to insert it in the db should be legal.

    match self.insert_fact(&fact) {
      Ok(true)   => FactCreated,
      Ok(false)  => FactExists,
      Err(e)     => FactFail(format!("{:?}", e))
    }
  }
  
  fn search_facts<'a>(&self, query : Vec<Clause>) -> SearchResponse<'a> {
    use fact_db::SearchResponse::*;
    use native_types::OHValue::*;
    
    //Check there is at least one clause
    if query.len() == 0 {
      return SearchInvalid("Empty search query".to_string());
    };
    
    //Check that clauses:
    // * Have sequential variables
    // * Reference predicates in the database
    // * Only unify variables of equal type
    {
      let mut var_type : Vec<HType> = Vec::new(); 
      for clause in query.iter() {
        let pred = match self.pred_by_name.get(&clause.pred_name) {
          Some(pred) => pred,
          None => return SearchInvalid(format!("{} is not a registered predicate.", clause.pred_name)),
        };
        for (idx, slot) in clause.args.iter().enumerate() {
          match slot {
              &MatchExpr::Unbound
            | &MatchExpr::HConst(_) => (),
              &MatchExpr::Var(v) => {
                let v = v as usize;
                if v == var_type.len() {
                  var_type.push(pred.types[idx])
                } else if v > var_type.len() {
                  return SearchInvalid(format!("Hole between {} and {} in variable numbering.", var_type.len() - 1, v));
                } else if var_type[v] != pred.types[idx] {
                  return SearchInvalid(format!("Variable {} attempt to unify incompatible types {:?} and {:?}", v, var_type[v], pred.types[idx]))
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
    for clause in query.iter() {
      let table_name = format!("facts.{}", clause.pred_name);
      let mut clause_elements = Vec::new();
      for (idx, arg) in clause.args.iter().enumerate() {
        match arg {
          &MatchExpr::Unbound => (),
          &MatchExpr::Var(var) => if var <= var_names.len() as u32 {
              var_names.push(
                format!("{}.arg{}", table_name, idx));
              var_types.push(self.pred_by_name[clause.pred_name].types[idx]);
            } else {
              let piece = format!("{}.arg{} = {}", table_name, idx, var_names[var as usize]);
              if idx == 0 {
                //For the first element, we have no ON clause, so stick this in WHERE
                where_clause.push(piece);
              } else {
                clause_elements.push(piece);
              }
            },
          &MatchExpr::HConst(ref val) => {
            vals.push(val);
            where_clause.push(
              format!("{}.arg{} = ${}", table_name, idx, vals.len()));
          }
        }
      }
      restricts.push(clause_elements);
      tables.push(table_name);
    }
    let vars = format!("({})", var_names.connect(", "));
    let main_table = tables.pop().unwrap();
    let main_join = restricts.pop();
    assert_eq!(main_join, Some(vec![]));
    let join_blocks : Vec<String> = tables.iter().zip(restricts.iter()).map(|(table, join)| {
        format!("JOIN {} ON {}", table, join.connect(" AND "))
      }).collect();
    let join_query = join_blocks.connect(" ");
    let res_stmt = self.conn.prepare(
      format!("SELECT {} FROM {} {} WHERE {}",
              vars, main_table, join_query,
              where_clause.connect(" AND ")).as_slice());
    let stmt = match res_stmt {
      Ok(stmt) => stmt,
      Err(e) => return SearchFail(
        format!("Preparing statement failed: {:?}", e))
    };
    let res_rows = stmt.query(vals.as_slice());
    let rows = match res_rows {
      Ok(rows) => rows,
      Err(e)   => return SearchFail(
        format!("Executing query failed: {:?}", e)) 
    };

    let anss : Vec<Vec<OHValue>> = rows.map(|row| {
      var_types.iter().enumerate().map(|(idx, h_type)| {
        match h_type {
          &HType::UInt64  => { 
            let v : i64 = row.get(idx);
            UInt64OV(v as u64)},
          &HType::HString => HStringOV(row.get(idx)),
          &HType::Blob    => BlobOV(row.get(idx))
        }
      }).collect() 
    }).collect();

    SearchAns(anss)
  }
}
