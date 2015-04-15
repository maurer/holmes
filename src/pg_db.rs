use native_types::*;
use native_types::HType::*;
use std::*;

use std::convert::From;
use std::fmt::{Formatter};
use fact_db::*;
use std::str::FromStr;

use postgres::{Connection, ConnectError, Error, SslMode};

use std::collections::HashSet;
use std::collections::hash_map::{HashMap};
use std::collections::hash_map::Entry::{Occupied, Vacant};

use postgres::ToSql;
use postgres::types::IsNull;
use std::sync::Arc;

use std::io::Write;

type ClauseId = (String, i32);

#[derive(Debug)]
pub enum DBError {
  ConnectError(ConnectError),
  Error(Error),
  TypeParseError(String),
  InternalError(String)
}
use pg_db::DBError::TypeParseError;
use pg_db::DBError::InternalError;

fn i32_cast(v : &u32) -> &i32 {
  unsafe {
    ::std::mem::transmute(v)
  }
}

impl From<Error> for DBError {
  fn from(x : Error) -> DBError {DBError::Error(x)}
}
impl From<ConnectError> for DBError {
  fn from(x : ConnectError) -> DBError {DBError::ConnectError(x)}
}

impl ::std::fmt::Display for DBError {
  fn fmt (&self, fmt : &mut Formatter) -> fmt::Result {
    match *self {
      DBError::ConnectError(ref x) => x.fmt(fmt),
      DBError::Error(ref x) => x.fmt(fmt),
      DBError::TypeParseError(ref s) =>
        fmt.write_str(&format!("Could not parse db type: {}",
                               s.clone())),
      DBError::InternalError(ref s) =>
        fmt.write_str(&format!("PgDB Internal Error: {}",
                               s.clone()))
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
  fn to_sql<W: ?Sized>(&self, ty: &::postgres::types::Type, out : &mut W) -> Result<IsNull, Error> 
    where Self: Sized, W: Write
  {
    self.to_string().to_sql(ty, out)
  }
  fn accepts(ty: &::postgres::types::Type) -> bool {
    String::accepts(ty)
  }
  fn to_sql_checked(&self, ty: &::postgres::types::Type, out: &mut Write) -> Result<IsNull, Error> {
    self.to_string().to_sql_checked(ty, out)
  }
}

impl ToSql for HValue {
  fn to_sql<W: ?Sized>(&self, ty: &::postgres::types::Type, out : &mut W) -> Result<IsNull, Error> 
    where Self: Sized, W: Write
  {
    use native_types::HValue::*;
    match *self {
      UInt64V(i)  => (i as i64).to_sql(ty, out),
      HStringV(ref s) => s.clone().to_sql(ty, out),
      BlobV(ref b)    => b.to_sql(ty, out),
    }
  }
  fn accepts(_ty: &::postgres::types::Type) -> bool {
     true // It varies wildly based on the type, so we approximate as yes
  }
  fn to_sql_checked(&self, ty: &::postgres::types::Type, out: &mut Write) -> Result<IsNull, Error> {
    use native_types::HValue::*;
    match *self {
      UInt64V(i)  => (i as i64).to_sql_checked(ty, out),
      HStringV(ref s) => s.clone().to_sql_checked(ty, out),
      BlobV(ref b)    => b.to_sql_checked(ty, out),
    }
  }
}

fn substitute(clause : &Clause, ans : &Vec<HValue>) -> Fact {
  use native_types::MatchExpr::*;
  Fact {
    pred_name : clause.pred_name.clone(),
    args : clause.args.iter().map(|slot| {
      match slot {
        &Unbound       => panic!("Unbound is not allowed in substituted facts"),
        &Var(ref n)    => ans[*n as usize].clone(),
        &HConst(ref v) => v.clone()
      }
    }).collect()
  }
}

pub struct PgDB {
  conn              : Connection,
  pred_by_name      : HashMap<String, Predicate>,
  insert_by_name    : HashMap<String, String>,
  rule_by_pred_name : HashMap<String, Vec<Arc<Rule>>>,
  rule_exec_cache   : HashMap<Rule, HashSet<Vec<HValue>>>
}

impl PgDB {
  pub fn new(conn_str : &str) -> Result<PgDB, DBError> {
    let conn = try!(Connection::connect(conn_str, &SslMode::None));
    
    //Create schemas
    try!(conn.execute("create schema if not exists facts", &[]));
    try!(conn.execute("create schema if not exists clauses", &[]));

    //Create Tables
    try!(conn.execute("create table if not exists predicates (pred_name varchar not null, ordinal int4 not null, type varchar not null)", &[]));
    try!(conn.execute("create table if not exists gen_clauses (id serial, pred_name varchar not null, tgt serial)", &[]));
    try!(conn.execute("create table if not exists rules (head_pred varchar not null, head_clause serial, body_pred varchar not null, body_clause serial)", &[]));

    //Create incremental PgDB object
    let mut pg_db = PgDB {
      conn : conn,
      pred_by_name      : HashMap::new(),
      insert_by_name    : HashMap::new(),
      rule_by_pred_name : HashMap::new(),
      rule_exec_cache   : HashMap::new(),
    };
   
    //Reload predicate cache
    let mut pred_by_name : HashMap<String, Predicate> = HashMap::new();
    {
      let pred_stmt = try!(pg_db.conn.prepare("select pred_name, type from predicates ORDER BY pred_name, ordinal"));
      let pred_types = try!(pred_stmt.query(&[]));
      for type_entry in pred_types {
        let name : String = type_entry.get(0);
        let h_type_str : String = type_entry.get(1);
        let h_type : HType = match FromStr::from_str(&h_type_str) {
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

    //Populate fact insert cache
    for pred in pred_by_name.values() {
      &pg_db.gen_insert_stmt(pred);
    }

    //Finish predicate cache
    pg_db.pred_by_name = pred_by_name;
 
    //Reload rule cache
    let rules = {
      let rule_stmt = try!(pg_db.conn.prepare("select head_clause, head_pred, body_clause, body_pred from rules"));
      let mut body_ids_by_head_id : HashMap<ClauseId, Vec<ClauseId>> = HashMap::new();
      let rows = try!(rule_stmt.query(&[]));
      for row in rows {
        let head : ClauseId = (row.get(1), row.get(0));
        let body : ClauseId = (row.get(3), row.get(2));
        match body_ids_by_head_id.entry(head) {
          Vacant(entry) => {entry.insert(vec![body]);}
          Occupied(mut entry) => entry.get_mut().push(body)
        }
      }
      try!(body_ids_by_head_id.iter().map(|(head_id, body_ids)| {
        let head_clause = try!(pg_db.get_clause(head_id));
        let body_clauses = try!(body_ids.iter().map(|body_id| {
          pg_db.get_clause(body_id)
        }).collect::<Result<Vec<Clause>,DBError>>());
        Ok(Rule {
          head : head_clause,
          body : body_clauses,
          wheres : vec![] //TODO fill in properly
        })
      }).collect::<Result<Vec<Rule>, DBError>>())
    };
    for rule in rules {
      &pg_db.add_rule_cache(&rule);
    }
    //TODO load exec cache
    
    Ok(pg_db)
  }

  fn get_clause(&self, clause_id : &ClauseId)
     -> Result<Clause, DBError> {
    use native_types::HValue::*;
    let stmt = try!(self.conn.prepare(&format!("select * from clauses.{} where id=$1", clause_id.0)));
    let rows = try!(stmt.query(&[&clause_id.1]));
    let row = rows.iter().next().expect("Should be one row");
    let args = self.pred_by_name[&clause_id.0].types.iter().enumerate().map(|(idx, h_type)| {
      let var_idx = idx * 2 + 1;
      let val_idx = var_idx + 1;
      let var : Option<i32> = row.get(var_idx);
      let val : Option<HValue> =
        match *h_type {
          UInt64  => row.get::<_,Option<i64>>(val_idx).map(|i|{UInt64V(i as u64)}),
          HString => row.get::<_,Option<String>>(val_idx).map(HStringV),
          Blob    => row.get::<_,Option<Vec<u8>>>(val_idx).map(BlobV)
        };
      match (var, val) {
        (None, None) => MatchExpr::Unbound,
        (Some(v), _) => MatchExpr::Var(v as u32),
        (_, Some(v)) => MatchExpr::HConst(v)
      }
    }).collect::<Vec<MatchExpr>>();
    Ok(Clause {
      pred_name : clause_id.0.clone(),
      args : args
    })
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

      table_str.push_str(&format!("arg{} {},", ordinal, h_type_to_sql_type(h_type)));
    }
    table_str.pop();
    table_str.push(')');

    let clause_str = format!("(id serial primary key, {})", types.iter().enumerate().map(|(idx, h_type)| {
      format!("var{} int4, val{} {}", idx, idx, h_type_to_sql_type(h_type))
    }).collect::<Vec<String>>().connect(", "));

    try!(self.conn.execute(&format!("create table facts.{} {}", name, table_str), &[]));
    try!(self.conn.execute(&format!("create table clauses.{} {}", name, clause_str), &[]));
    return Ok(());
  }

  fn insert_fact(&mut self, fact : &Fact) -> Result<bool, DBError> {
    let stmt : String = try!(self.insert_by_name
      .get(&fact.pred_name)
      .ok_or(InternalError("Insert Statement Missing"
                           .to_string()))).clone();
    let argrefs : Vec<&ToSql> = fact.args.iter().map(|x|{x as &ToSql}).collect();
    let inserted = try!(self.conn.execute(&stmt, &argrefs)) > 0;
    if inserted {
      let rules = match self.rule_by_pred_name.get(&fact.pred_name) {
        Some(z) => z.clone(),
        None => Vec::new()
      };
      for rule in rules {
        self.run_rule(&rule);
      }
    }
    Ok(inserted)
  }

  fn insert_clause(&mut self, clause : &Clause) -> Result<ClauseId, DBError> {
   let table = clause.pred_name.clone();
   let (columns, traits) : (Vec<String>, Vec<&ToSql>) = 
     clause.args.iter().enumerate().filter_map(|(idx, arg)| {
     match *arg {
       MatchExpr::Unbound       => None,
       MatchExpr::Var(ref v)    => {
         Some((format!("var{}", idx), i32_cast(v) as &ToSql))
       }
       MatchExpr::HConst(ref c) => Some((format!("val{}", idx), c as &ToSql))
     }}).unzip();
   let template = columns.iter().enumerate().map(|(idx, _)| {
     format!("${}", idx + 1)
   }).collect::<Vec<String>>().connect(", ");
   let stmt = try!(self.conn.prepare(
     &format!("insert into clauses.{} ({}) values ({}) returning id",
              table, columns.connect(", "), template)));
   let res = try!(stmt.query(&traits));
   Ok((table, res.iter().next().expect("Clause insert failure").get(0)))
  }

  fn run_rule(&mut self, rule : &Rule) {
    match self.search_facts(&rule.body) {
      SearchResponse::SearchAns(anss) => {
        //TODO: make this persist the cache
        for ans in anss {
          let miss = match self.rule_exec_cache.entry(rule.clone()) {
            Vacant(entry) => {
              let mut cache = HashSet::new();
              cache.insert(ans.clone());
              entry.insert(cache);
              true
            }
            Occupied(mut entry) => {
              let miss = !entry.get().contains(&ans);
              entry.get_mut().insert(ans.clone());
              miss
            }
          };
          if miss {
            assert!(self.insert_fact(&substitute(&rule.head, &ans)).is_ok());
          }
        }
      }
      SearchResponse::SearchInvalid(s) => panic!("Internal invalid search query {}", s),
      SearchResponse::SearchFail(s) => panic!("Search procedure failure {}", s),
      SearchResponse::SearchNone => ()
    }
  }

  fn add_rule_cache(&mut self, rule : &Rule) {
    let a_rule : Arc<Rule> = Arc::new(rule.clone());
    for pred in &rule.body {
      match self.rule_by_pred_name.entry(pred.pred_name.clone()) {
        Vacant(entry) => {entry.insert(vec![a_rule.clone()]);}
        Occupied(mut entry) => entry.get_mut().push(a_rule.clone())
      }
    }
  }

  fn insert_rule(&mut self, head_id : ClauseId, body_ids : Vec<ClauseId>) -> Result<(), DBError> {
    //XXX: This should probably be in a txn, an error will create a malformed rule on reboot
    for body_id in body_ids.iter() {
      try!(self.conn.execute(
         &format!("insert into rules values ($1, $2, $3, $4)"),
         &[&head_id.0 as &ToSql,
           &head_id.1 as &ToSql,
           &body_id.0 as &ToSql,
           &body_id.1 as &ToSql]));
    }
    Ok(())
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
          //TODO use zip that mandates same length iters?
          return FactTypeMismatch;
        }
      }
      None => return FactPredUnreg(fact.pred_name.to_string())
    }
    // We know about the predicate, and the types match, so
    // attempting to insert it in the db should be legal.

    match self.insert_fact(&fact) {
      Ok(true)   => {
        let rules = match self.rule_by_pred_name.get(&fact.pred_name) {
          Some(z) => z.clone(),
          None => Vec::new()
        };
        for rule in rules {
          self.run_rule(&rule);
        }
        FactCreated
      }
      Ok(false)  => FactExists,
      Err(e)     => FactFail(format!("{:?}", e))
    }
  }
  
  fn search_facts(&self, query : &Vec<Clause>) -> SearchResponse {
    use fact_db::SearchResponse::*;
    use native_types::HValue::*;

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
    for (idxc, clause) in query.iter().enumerate() {
      let table_name = format!("facts.{}", clause.pred_name);
      let alias_name = format!("t{}", idxc);
      let mut clause_elements = Vec::new();
      for (idx, arg) in clause.args.iter().enumerate() {
        match arg {
          &MatchExpr::Unbound => (),
          &MatchExpr::Var(var) => if var >= var_names.len() as u32 {
              var_names.push(
                format!("{}.arg{}", alias_name, idx));
              var_types.push(self.pred_by_name[&clause.pred_name].types[idx]);
            } else {
              let piece = format!("{}.arg{} = {}", alias_name, idx, var_names[var as usize]);
              if idxc == 0 {
                //For the first element, we have no ON clause, so stick this in WHERE
                where_clause.push(piece);
              } else {
                clause_elements.push(piece);
              }
            },
          &MatchExpr::HConst(ref val) => {
            vals.push(val);
            where_clause.push(
              format!("{}.arg{} = ${}", alias_name, idx, vals.len()));
          }
        }
      }
      restricts.push(clause_elements);
      tables.push(format!("{} as {}", table_name, alias_name));
    }
    let vars = format!("{}", var_names.connect(", "));
    tables.reverse();
    restricts.reverse();
    let main_table = tables.pop().unwrap();
    let main_join = restricts.pop();
    assert_eq!(main_join, Some(vec![]));
    let join_blocks : Vec<String> = tables.iter().zip(restricts.iter()).map(|(table, join)| {
        if join.len() == 0 {
          format!("JOIN {} ", table)
        } else {
          format!("JOIN {} ON {}", table, join.connect(" AND "))
        }
      }).collect();
    let join_query = join_blocks.connect(" ");
    let where_clause = {
      if where_clause.len() == 0 {
        String::new()
      } else {
        format!("WHERE {}", where_clause.connect(" AND "))
      }
    };
    let raw_stmt = 
      format!("SELECT {} FROM {} {} {}",
              vars, main_table, join_query,
              where_clause);
    let res_stmt = self.conn.prepare(&raw_stmt);
    let stmt = match res_stmt {
      Ok(stmt) => stmt,
      Err(e) => return SearchFail(
        format!("Preparing statement failed: {}\n{:?}", raw_stmt, e))
    };
    let res_rows = stmt.query(&vals);
    let rows = match res_rows {
      Ok(rows) => rows,
      Err(e)   => return SearchFail(
        format!("Executing query failed: {:?}", e)) 
    };

    let mut anss : Vec<Vec<HValue>> = rows.iter().map(|row| {
      var_types.iter().enumerate().map(|(idx, h_type)| {
        match h_type {
          &HType::UInt64  => { 
            let v : i64 = row.get(idx);
            UInt64V(v as u64)},
          &HType::HString => HStringV(row.get(idx)),
          &HType::Blob    => BlobV(row.get(idx))
        }
      }).collect() 
    }).collect();
    anss.dedup();
    SearchAns(anss)
  }

  fn new_rule(&mut self, rule : Rule) -> RuleResponse {
    use fact_db::RuleResponse::*;
    
    //Persist the rule
    //Create clauses
    let head_id : ClauseId = match self.insert_clause(&rule.head) {
      Ok(v) => v,
      Err(e) => return RuleFail(format!("Issue creating head clause: {:?}", e))
    };
    let body_ids : Vec<ClauseId> =
      match rule.body.iter().map(|x|{self.insert_clause(x)}).collect::<Result<Vec<ClauseId>, DBError>>() {
        Ok(v) => v,
        Err(e) => return RuleFail(format!("Issue creating body clauses: {:?}", e))
      };
    //Create rule
    match self.insert_rule(head_id, body_ids) {
      Err(e) => return RuleFail(format!("Issue creating rule: {:?}", e)),
      _ => ()
    }

    //Enter rule into rulecache
    self.add_rule_cache(&rule);

    //Actually run the rule?
    self.run_rule(&rule);

    RuleAdded
  }
}
