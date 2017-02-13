//! Postgres-based Fact Database
//!
//! # Design Notes
//!
//! ## Scope
//!
//! The general philsophy is that things having to do with persistence go here,
//! while things related to non-persistent components go in `holmes::engine`.
//!
//! In the long run, we would like to persist nearly everything in the
//! database, so that a client-server model can one bay restored. However,
//! in the short term this has little benefit, so only the items needing to
//! use SQL for efficient execution are being included.
//!
//! The biggest hurdle here is the persistence of code:
//!
//! * How do we store Types?
//! * How do we store bound functions?
//!
//! One possible long term answer is Cap'n' Proto `SturdyRef`s
//!
//! ## Other Databases
//!
//! For the moment, this is the only implementation, and there are no others
//! on the horizon, so this interface is not abstract.
//!
//! The only major hurdle to using another backend would be figuring out how
//! to make the `dyn` module abstract over databases.
use std::collections::hash_map::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use postgres::{rows, Connection, TlsMode};
use postgres::params::IntoConnectParams;
use postgres::types::{FromSql, ToSql};

use engine::types::{Fact, Predicate, Field, MatchExpr, Clause, Projection};
use std::cell::RefCell;
use std::time::Instant;
use std::sync::Arc;

pub mod dyn;

#[allow(missing_docs)]
mod errors {
    use postgres as pg;
    error_chain! {
            errors {
                UriParse {
                    description("Postgres URI Parse Error")
                }
                Type(msg: String) {
                    description("Type Error")
                    display("Type Error: {}", msg)
                }
                Internal(msg: String) {
                    description("PgDB Internal Error")
                    display("PgDB Internal Error: {}", msg)
                }
                Arg(msg: String) {
                    description("Bad argument")
                    display("Bad argument: {}", msg)
                }
            }
            foreign_links {
                Connect(pg::error::ConnectError);
                Db(pg::error::Error);
            }
    }
}

pub use self::errors::*;

use self::dyn::types;
use self::dyn::{Type, Value};
use fact_db::{FactDB, FactId, CacheId};

fn db_expr(e: &Projection, names: &Vec<String>, table: &String) -> String {
    match *e {
        Projection::U64(v) => format!("{}", v),
        Projection::Var(v) => format!("{}", names[v]),
        Projection::Slot(n) => format!("{}.arg{}", table, n),
        Projection::SubStr { ref buf, ref start_idx, ref end_idx } => {
            format!("substring({} from CAST({} as INT) + 1 for CAST({} as INT) - CAST({} AS \
                     INT) + 1)",
                    db_expr(buf, names, table),
                    db_expr(start_idx, names, table),
                    db_expr(end_idx, names, table),
                    db_expr(start_idx, names, table))
        }
    }
}

fn db_type(e: &Projection, fields: &Vec<Field>, var_types: &Vec<Type>) -> Result<Type> {
    match *e {
        Projection::U64(_) => Ok(Arc::new(types::UInt64)),
        Projection::Var(v) => Ok(var_types[v].clone()),
        Projection::Slot(n) => Ok(fields[n].type_.clone()),
        Projection::SubStr { ref buf, ref start_idx, ref end_idx } => {
            let buf_type = db_type(&buf, fields, var_types)?;
            if buf_type != Arc::new(types::String) && buf_type != Arc::new(types::Bytes) &&
               buf_type != Arc::new(types::LargeBytes) {
                bail!(ErrorKind::Type(format!("Tried to take substring of non-string or bytes \
                                               type: {:?} : {:?}",
                                              buf,
                                              buf_type)))
            }
            let start_type = db_type(&start_idx, fields, var_types)?;
            let end_type = db_type(&start_idx, fields, var_types)?;
            if start_type != Arc::new(types::UInt64) {
                bail!(ErrorKind::Type(format!("Tried to index starting with non-numeric type: \
                                               {:?} : {:?}",
                                              start_idx,
                                              start_type)))
            }
            if end_type != Arc::new(types::UInt64) {
                bail!(ErrorKind::Type(format!("Tried to index ending with non-numeric type: \
                                               {:?} : {:?}",
                                              end_idx,
                                              end_type)))
            }
            Ok(buf_type)
        }
    }
}

/// An iterator over a `postgres::rows::Row`.
/// It does not implement the normal iter interface because it does not have
/// a set item type, but it implements a similar interface for ease of use.
pub struct RowIter<'a> {
    row: &'a rows::Row<'a>,
    index: usize,
}

impl<'a> RowIter<'a> {
    /// Create a new row iterator starting at the beginning of the provided row
    pub fn new(row: &'a rows::Row) -> Self {
        RowIter {
            row: row,
            index: 0,
        }
    }
    /// Gets the next item in the row, using a `FromSql` instance to read it.
    /// If there is not a next item, returns `None`
    pub fn next<T>(&mut self) -> Option<T>
        where T: FromSql
    {
        let idx = self.index;
        self.index += 1;
        self.row.get(idx)
    }
}

/// Object representing a postgres-backed fact database instance
pub struct PgDB {
    conn: Connection,
    pred_by_name: RefCell<HashMap<String, Predicate>>,
    insert_by_name: RefCell<HashMap<String, String>>,
    named_types: RefCell<HashMap<String, Type>>,
}

impl PgDB {
    /// Create a new PgDB object by passing in a Postgres connection string
    // TODO Add type parameters to call?
    // At the moment, persistence with custom types will result in failures
    // on a reconnect, so use a fresh database every time.
    // There's not a good way to persist custom types, so that fix will likely
    // come with optional parameters to seed types in at db startup.
    // TODO Should we be passing in a Connection object rather than a string?
    pub fn new(uri: &str) -> Result<PgDB> {
        // Create database if it doesn't already exist and we can
        // TODO do this only on connection failure?
        let mut params = try!(uri.into_connect_params()
            .map_err(|_| ErrorKind::UriParse));
        match params.database.clone() {
            Some(db) => {
                params.database = Some("postgres".to_owned());
                let conn = try!(Connection::connect(params, TlsMode::None));
                let create_query = format!("CREATE DATABASE {}", &db);
                // TODO only suppress db exists error
                let _ = conn.execute(&create_query, &[]);
            }
            None => (),
        }
        // Establish the connection
        let conn = try!(Connection::connect(uri, TlsMode::None));

        // Create schemas
        try!(conn.execute("create schema if not exists facts", &[]));
        try!(conn.execute("create schema if not exists cache", &[]));

        // Create Tables
        try!(conn.execute("create table if not exists predicates (id serial primary key, \
                           name varchar not null, \
                           description varchar)",
                          &[]));
        try!(conn.execute("create table if not exists fields (\
                           pred_id serial references predicates(id), \
                           ordinal int4 not null, \
                           type varchar not null, \
                           name varchar, \
                           description varchar)",
                          &[]));
        try!(conn.execute("create table if not exists rules (id serial primary key , rule varchar \
                      not null)",
                     &[]));
        try!(conn.execute("create sequence if not exists cache_id", &[]));

        // Create incremental PgDB object
        let db = PgDB {
            conn: conn,
            pred_by_name: RefCell::new(HashMap::new()),
            insert_by_name: RefCell::new(HashMap::new()),
            named_types: RefCell::new(types::default_types()
                .iter()
                .filter_map(|type_| type_.name().map(|name| (name.to_owned(), type_.clone())))
                .collect()),
        };

        try!(db.rebuild_predicate_cache());

        Ok(db)
    }

    /// Kick everyone off the database and destroy the data at the provided URI
    pub fn destroy(uri: &str) -> Result<()> {
        let mut params = try!(uri.into_connect_params()
            .map_err(|_| ErrorKind::UriParse));
        let old_db = try!(params.database
            .ok_or_else(||
                    ErrorKind::Arg(format!(
                            "No database specified to destroy in {}.", uri))));
        params.database = Some("postgres".to_owned());
        let conn = try!(Connection::connect(params, TlsMode::None));
        let disco_query = format!("SELECT pg_terminate_backend(pg_stat_activity.pid) FROM \
                                   pg_stat_activity WHERE pg_stat_activity.datname = '{}' AND \
                                   pid <> pg_backend_pid()",
                                  &old_db);
        try!(conn.execute(&disco_query, &[]));
        let drop_query = format!("DROP DATABASE {}", &old_db);
        try!(conn.execute(&drop_query, &[]));
        Ok(())
    }


    // Rebuilds the predicate cache
    // I'm assuming for the moment that there isn't going to be a lot of
    // dynamic type adding/removal, and so rebuilding the predicate/insert
    // statement cache on add/remove isn't a big deal
    fn rebuild_predicate_cache(&self) -> Result<()> {
        *self.pred_by_name.borrow_mut() = HashMap::new();
        *self.insert_by_name.borrow_mut() = HashMap::new();
        {
            // Scoped borrow of connection
            let pred_stmt = try!(self.conn
                .prepare("select predicates.name, predicates.description, fields.name, \
                          fields.description, fields.type from predicates JOIN fields ON \
                          predicates.id = fields.pred_id ORDER BY predicates.id, fields.ordinal"));
            let pred_types = try!(pred_stmt.query(&[]));
            for type_entry in pred_types.iter() {
                let mut row = RowIter::new(&type_entry);
                let name: String = row.next().unwrap();
                // TODO: there's funny layering of nested options issues here
                let pred_descr: Option<String> = row.next();
                let field_name: Option<String> = row.next();
                let field_descr: Option<String> = row.next();
                let h_type_str: String = row.next().unwrap();
                let h_type = match self.get_type(&h_type_str) {
                    Some(ty) => ty,
                    None => types::Trap::new(),
                };
                let field = Field {
                    name: field_name,
                    description: field_descr,
                    type_: h_type.clone(),
                };
                match self.pred_by_name.borrow_mut().entry(name.clone()) {
                    Vacant(entry) => {
                        entry.insert(Predicate {
                            name: name.clone(),
                            description: pred_descr,
                            fields: vec![field],
                        });
                    }
                    Occupied(mut entry) => {
                        entry.get_mut().fields.push(field);
                    }
                }
            }
        }
        // Populate fact insert cache
        self.pred_by_name.borrow().values().inspect(|pred| self.gen_insert_stmt(pred)).count();
        Ok(())
    }

    // Generates a prebuilt insert statement for a given predicate, and stores
    // it in the cache so we don't have to rebuild it every time.
    // TODO: Is it possible for these to be stored prepared statements somehow?
    // TODO: There might be an issue here with types with multifield width?
    fn gen_insert_stmt(&self, pred: &Predicate) {
        let args: Vec<String> = pred.fields
            .iter()
            .enumerate()
            .map(|(k, _)| format!("${}", k + 1))
            .collect();
        let stmt = format!("insert into facts.{} values (DEFAULT, {}) ON \
                            CONFLICT DO NOTHING",
                           pred.name,
                           args.join(", "));
        self.insert_by_name.borrow_mut().insert(pred.name.clone(), stmt);
    }

    // Persist a predicate into the database
    // This function is internal because it does not add it to the object, it
    // _only_ puts record of the predicate into the database.
    fn insert_predicate(&self, pred: &Predicate) -> Result<()> {
        let &Predicate { ref name, ref description, ref fields } = pred;
        let stmt = self.conn
            .prepare("insert into predicates (name, description) values ($1, $2) returning id")?;
        let pred_id: i32 = stmt.query(&[name, description])?.get(0).get(0);
        for (ordinal, field) in fields.iter().enumerate() {
            try!(self.conn
                .execute("insert into fields (pred_id, name, description, type, ordinal) \
                          values ($1, $2, $3, $4, $5)",
                         &[&pred_id,
                           &field.name,
                           &field.description,
                           &field.type_
                               .name()
                               .ok_or(ErrorKind::Arg("Field type had no name".to_string()))?,
                           &(ordinal as i32)]));
        }
        let table_str = fields.iter()
            .flat_map(|field| field.type_.repr())
            .enumerate()
            .map(|(ord, repr)| format!("arg{} {}", ord, repr))
            .collect::<Vec<_>>()
            .join(", ");
        let col_str = fields.iter()
            .flat_map(|field| {
                field.type_
                    .repr()
                    .iter()
                    .map(|_| field.type_.large_unique())
                    .collect::<Vec<_>>()
            })
            .enumerate()
            .filter(|&(_, x)| !x)
            .map(|(ord, _)| format!("arg{}", ord))
            .collect::<Vec<_>>()
            .join(", ");
        self.conn
            .execute(&format!("create table facts.{} (id serial primary \
                               key, {}, unique({}))",
                              name,
                              table_str,
                              col_str),
                     &[])?;
        Ok(())
    }

    fn cache_hit(&self, cache: CacheId, facts: Vec<FactId>) -> Result<()> {
        let borrow: Vec<&ToSql> = facts.iter().map(|x| x as &ToSql).collect();
        try!(self.conn
            .execute(&format!("insert into cache.rule{} values ({})",
                              cache,
                              facts.iter()
                                  .enumerate()
                                  .map(|(x, _)| format!("${}", x + 1))
                                  .collect::<Vec<_>>()
                                  .join(", ")),
                     borrow.as_slice()));
        Ok(())
    }
}
impl FactDB for PgDB {
    type Error = Error;
    fn new_rule_cache(&self, clause: &Vec<Clause>) -> Result<CacheId> {
        let preds: Vec<String> = clause.iter().map(|x| x.pred_name.clone()).collect();
        let cache_stmt = try!(self.conn.prepare("select nextval('cache_id')"));
        let cache_res = try!(cache_stmt.query(&[]));
        let cache_id = cache_res.get(0).get(0);
        try!(self.conn.execute(&format!("create table cache.rule{} ({})",
                                        cache_id,
                                        preds.into_iter()
                                            .enumerate()
                                            .map(|(n, pred)| {
                                                format!("id{} serial references facts.{}(id)",
                                                        n,
                                                        pred)
                                            })
                                            .collect::<Vec<_>>()
                                            .join(", ")),
                               &[]));
        Ok(cache_id)
    }
    /// Adds a new fact to the database, returning false if the fact was already
    /// present in the database, and true if it was inserted.
    fn insert_fact(&self, fact: &Fact) -> Result<bool> {
        let stmt: String = try!(self.insert_by_name
                .borrow()
                .get(&fact.pred_name)
                .ok_or_else(|| ErrorKind::Internal("Insert Statement Missing".to_string())))
            .clone();
        Ok(try!(self.conn.execute(&stmt,
                                  &fact.args
                                      .iter()
                                      .flat_map(|x| x.to_sql().into_iter())
                                      .collect::<Vec<_>>())) > 0)
    }

    /// Registers a new type with the database.
    /// This is unstable, and will likely need to be moved to the initialization
    /// of the database object in order to allow reconnecting to an existing
    /// database.
    fn add_type(&self, type_: Type) -> Result<()> {
        let name = type_.name()
            .ok_or(ErrorKind::Arg("Tried to add a type with no name".to_string()))?;
        if !self.named_types.borrow().contains_key(name) {
            self.named_types.borrow_mut().insert(name.to_owned(), type_.clone());
            self.rebuild_predicate_cache()
        } else {
            bail!(ErrorKind::Type(format!("{} already registered", name)))
        }
    }

    /// Looks for a named type in the database's registry.
    /// This function is primarily useful for the DSL shorthand for constructing
    /// queries, since it allows you to use names of types when declaring
    /// functions rather than type objects.
    fn get_type(&self, type_str: &str) -> Option<Type> {
        self.named_types.borrow().get(type_str).map(|x| x.clone())
    }

    /// Fetches a predicate by name
    fn get_predicate(&self, pred_name: &str) -> Option<Predicate> {
        self.pred_by_name.borrow().get(pred_name).cloned()
    }

    /// Persists a predicate by name
    /// The name *must* consist only of lower case ASCII and _, anything else
    /// will be rejected. This restriction is because the predicate name is
    /// currently used to construct the table name.
    ///
    /// In the future, this restriction could be lifted by generating table
    /// names rather than using the names of predicates, but this helps a lot
    /// with debugging for now.
    // TODO lift restriction on predicate names
    fn new_predicate(&self, pred: &Predicate) -> Result<()> {
        // The predicate name is used as a table name, check it for legality
        if !valid_name(&pred.name) {
            bail!(ErrorKind::Arg("Invalid name: Use lowercase and \
                                 underscores only"
                .to_string()));
        }
        // If this predicate was already registered, check for a match
        match self.pred_by_name.borrow().get(&pred.name) {
            Some(existing) => {
                if existing != pred {
                    bail!(ErrorKind::Arg(format!("Predicate {} already registered at a \
                                                  different type.\nExisting: {:?}\nNew: {:?}",
                                                 &pred.name,
                                                 existing,
                                                 pred)));
                } else {
                    return Ok(());
                }
            }
            None => (),
        }

        try!(self.insert_predicate(&pred));
        self.gen_insert_stmt(&pred);
        self.pred_by_name.borrow_mut().insert(pred.name.clone(), pred.clone());
        Ok(())
    }

    /// Attempt to match the right hand side of a datalog rule against the
    /// database, returning a list of solution assignments to the bound
    /// variables.
    fn search_facts(&self,
                    query: &Vec<Clause>,
                    cache: Option<CacheId>)
                    -> Result<Vec<(Vec<FactId>, Vec<Value>)>> {
        let cache_clause = match cache {
            Some(cache_id) => {
                format!("not exists (select 1 from cache.rule{} WHERE {})",
                        cache_id,
                        query.iter()
                            .enumerate()
                            .map(|(n, _)| format!("id{} = t{}.id", n, n))
                            .collect::<Vec<_>>()
                            .join(" AND "))
            }
            None => format!("1 = 1"),
        };
        // Check there is at least one clause
        if query.len() == 0 {
            bail!(ErrorKind::Arg("Empty search query".to_string()));
        };

        // Check that clauses:
        // * Have sequential variables
        // * Reference predicates in the database
        // * Only unify variables of equal type
        {
            let mut var_type: Vec<Type> = Vec::new();
            for clause in query.iter() {
                let pred = match self.pred_by_name.borrow().get(&clause.pred_name).cloned() {
                    Some(pred) => pred,
                    None => {
                        bail!(ErrorKind::Arg(format!("{} is not a registered predicate.",
                                                     clause.pred_name)))
                    }
                };
                for &(ref proj, ref binding) in clause.args.iter() {
                    match *binding {
                        MatchExpr::Unbound |
                        MatchExpr::Const(_) => (),
                        MatchExpr::Var(v) => {
                            let v = v as usize;
                            let type_ = db_type(proj, &pred.fields, &var_type)?;
                            if v == var_type.len() {
                                var_type.push(type_)
                            } else if v > var_type.len() {
                                bail!(ErrorKind::Arg(format!("Hole between {} and {} in \
                                                              variable numbering.",
                                                             var_type.len() - 1,
                                                             v)));
                            } else if &var_type[v] != &type_ {
                                bail!(ErrorKind::Arg(format!("Variable {} attempt to unify \
                                                              incompatible types {:?} and {:?}",
                                                             v,
                                                             var_type[v],
                                                             type_)));
                            }
                        }
                    }
                }
            }
        }

        // Actually build and execute the query
        let mut tables = Vec::new();    // Predicate names involved in the query,
                                    // in the sequence they appear
        let mut restricts = Vec::new(); // Unification expressions, indexed by
                                    // which join they belong on.
        let mut var_names = Vec::new(); // Translation of variable numbers to
                                    // sql exprs
        let mut fact_ids = Vec::new(); // Translation of fact ids to sql exprs
        let mut var_types = Vec::new(); // Translation of variable numbers to
                                    // Types
        let mut vals: Vec<&ToSql> = Vec::new(); // Values to be quoted into the
                                             // prepared statement

        for (idxc, clause) in query.iter().enumerate() {
            // The clause refers to a table named by the predicate
            let table_name = format!("facts.{}", clause.pred_name);
            // We will refer to it by a numbered alias, to make joining easier
            let alias_name = format!("t{}", idxc);
            let pred = self.pred_by_name.borrow().get(&clause.pred_name).unwrap().clone();
            fact_ids.push(format!("{}.id", alias_name));
            let mut clause_elements = Vec::new();
            for &(ref proj, ref arg) in clause.args.iter() {
                let proj_str = db_expr(&proj, &var_names, &alias_name);
                match *arg {
                    MatchExpr::Unbound => (),
                    MatchExpr::Var(var) => {
                        if var >= var_names.len() {
                            // This situation means it's the first occurrence of the variable
                            // We record this definition as the canonical definition for use
                            // in the select, and store the type to know how to extract it.
                            var_names.push(proj_str);
                            let type_ = db_type(proj, &pred.fields, &var_types)?;
                            var_types.push(type_);
                        } else {
                            // The variable has occurred correctly, so we add it being equal
                            // to the canonical definition to the join clause for this table
                            let piece = format!("{} = {}", proj_str, var_names[var]);
                            clause_elements.push(piece);
                        }
                    }
                    MatchExpr::Const(ref val) => {
                        // Since we're comparing against a constant, this restriction can
                        // go in the where clause.
                        // I stash the value in a buffer for later use with the prepared
                        // statement, and put the index into the buffer into the where
                        // clause chunk.
                        vals.extend(val.to_sql());
                        restricts.push(format!("{} = ${}", proj_str, vals.len()));
                    }
                }
            }
            restricts.extend(clause_elements);
            tables.push(format!("{} as {}", table_name, alias_name));
        }
        // Make sure we're never empty on bound variables. If we are, we will get
        // SELECT FROM
        // which will not work.
        var_names.push("0".to_string());

        let mut merge_vars = fact_ids.clone();

        merge_vars.extend(var_names.into_iter());

        let vars = format!("{}", merge_vars.join(", "));
        tables.reverse();
        restricts.reverse();
        let main_table = tables.pop()
            .ok_or(ErrorKind::Internal(format!("Match clause accesses no tables")))?;
        let join_query = tables.iter()
            .map(|table| format!("JOIN {} ON true", table))
            .collect::<Vec<_>>()
            .join(" ");
        restricts.push(cache_clause);
        let where_clause = format!("WHERE {}", restricts.join(" AND "));
        let raw_stmt = format!("SELECT {} FROM {} {} {}",
                               vars,
                               main_table,
                               join_query,
                               where_clause);
        trace!("search_facts: {}", raw_stmt);
        let db_check = Instant::now();
        let rows = try!(self.conn.query(&raw_stmt, &vals));
        trace!("search_facts query_time: {:?}", db_check.elapsed());
        trace!("search_facts: got {} rows", rows.len());
        rows.iter()
            .map(|row| {
                let mut row_iter = RowIter::new(&row);
                let mut ids = Vec::new();
                for _ in fact_ids.iter() {
                    match row_iter.next() {
                        Some(e) => ids.push(e),
                        None => {
                            bail!(ErrorKind::Internal(format!("Failure loading fact ids from row")))
                        }
                    }
                }
                let mut vars = Vec::new();
                for var_type in var_types.iter() {
                    match var_type.extract(&mut row_iter) {
                        Some(e) => vars.push(e),
                        None => bail!(ErrorKind::Internal(format!("Failure loading var from row"))),
                    }
                }
                // TODO this is not atomic with any error conditions, so if an error is hit
                // loading another row, the cache will remain hit. When I add transactions, I
                // should make this part of the transaction, and it should commit at the end
                // Also, the cache should be updated in a single batch statement at he end of the
                // loop for perf
                match cache {
                    Some(cache_id) => self.cache_hit(cache_id, ids.clone())?,
                    None => (),
                }
                Ok((ids, vars))
            })
            .collect()
    }
}

fn valid_name(name: &String) -> bool {
    name.chars().all(|ch| match ch {
        'a'...'z' | '_' => true,
        _ => false,
    })
}
