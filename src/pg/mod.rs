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
use r2d2_postgres::{PostgresConnectionManager, TlsMode};
use r2d2;

use fallible_iterator::FallibleIterator;

use postgres;
use postgres::{Connection, rows};
use postgres::rows::LazyRows;
use postgres::stmt::Statement;
use postgres::transaction::Transaction;
use postgres::params::IntoConnectParams;
use postgres::params;
use postgres::types::FromSql;

use engine::types::{Clause, Fact, Field, MatchExpr, Predicate};
use std::cell::RefCell;

pub mod dyn;

#[allow(missing_docs)]
mod errors {
    use postgres as pg;
    use r2d2;
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
                Db(pg::error::Error);
                R2D2Init(r2d2::InitializationError);
                R2D2Get(r2d2::GetTimeout);
            }
    }
}

pub use self::errors::*;

use self::dyn::types;
use self::dyn::{Type, Value};

/// FactId is intended as a database-wide identifier for a fact - they are unique across tables and
/// are intended for caching already run rules and recording providence.
pub type FactId = i64;

/// An iterator over a `postgres::rows::Row`.
/// It does not implement the normal iter interface because it does not have
/// a set item type, but it implements a similar interface for ease of use.
pub struct RowIter<'a> {
    row: &'a rows::Row<'a>,
    index: usize,
}

/// A prepared query within a transaction.
/// This abstraction is primarily to satisfy lifetime bounds during a lazy query.
pub struct Query<'trans, 'stmt> {
    stmt: Statement<'stmt>,
    trans: &'trans Transaction<'trans>,
    vals: Vec<Value>,
    fact_ids: usize,
    var_types: Vec<Type>,
}

impl<'trans, 'stmt> Query<'trans, 'stmt> {
    /// Actually runs the query stored inside, transforming it into a lazy query iterator
    pub fn run(&self) -> QueryIter {
        let sql: Vec<_> = self.vals.iter().flat_map(|x| x.to_sql()).collect();
        trace!("Starting incremental query");
        let rows = self.stmt.lazy_query(self.trans, &sql, 16384).unwrap();
        trace!("Incremental query returned");
        QueryIter {
            rows: rows,
            fact_ids: self.fact_ids,
            var_types: self.var_types.clone(),
        }
    }
    /// Gives the max Fact ID that this query can see
    pub fn fact_id(&self) -> FactId {
        // This is super incorrect in a threaded world - another transaction could
        // advance the 'fact_id' sequence while we don't have their facts in the read snapshot.
        // Luckily, we're async, not threaded atm, so this should be safe, just leave some holes in
        // the fact_id sequence, which I'm not terribly choked up about.
        match self.trans
            .query("select nextval('fact_id')", &[])
            .unwrap()
            .get(0)
            .get(0) {
            Some(fact_id) => fact_id,
            _ => 0,
        }
    }
}

/// A lazy query in the process of running. This iterator yields the result rows, one at a time, as
/// vectors of values labeled with fact IDs used as a source for them.
pub struct QueryIter<'trans, 'stmt> {
    rows: LazyRows<'trans, 'stmt>,
    fact_ids: usize,
    var_types: Vec<Type>,
}

impl<'trans, 'stmt> Iterator for QueryIter<'trans, 'stmt> {
    type Item = (Vec<FactId>, Vec<Value>);
    fn next(&mut self) -> Option<(Vec<FactId>, Vec<Value>)> {
        match self.rows.next().unwrap() {
            None => None,
            Some(row) => {
                let mut row_iter = RowIter::new(&row);
                let mut ids = Vec::new();
                for _ in 0..self.fact_ids {
                    match row_iter.next() {
                        Some(e) => ids.push(e),
                        None => panic!("Failure loading fact ids from row"),
                    }
                }
                let mut vars = Vec::new();
                for var_type in self.var_types.iter() {
                    match var_type.extract(&mut row_iter) {
                        Some(e) => vars.push(e),
                        None => panic!("Failure loading var from row"),
                    }
                }
                Some((ids, vars))
            }
        }
    }
}

impl<'a> RowIter<'a> {
    /// Create a new row iterator starting at the beginning of the provided row
    pub fn new(row: &'a rows::Row) -> Self {
        RowIter { row: row, index: 0 }
    }
    /// Gets the next item in the row, using a `FromSql` instance to read it.
    /// If there is not a next item, returns `None`
    pub fn next<T>(&mut self) -> Option<T>
    where
        T: FromSql,
    {
        let idx = self.index;
        self.index += 1;
        self.row.get(idx)
    }
}

fn param_into_builder(params: &params::ConnectParams) -> params::Builder {
    let user = params.user().unwrap();
    let mut builder = params::ConnectParams::builder();
    builder.port(params.port()).user(
        user.name(),
        user.password(),
    );
    params.database().map(|db| builder.database(db));
    builder
}

/// Object representing a postgres-backed fact database instance
pub struct PgDB {
    conn_pool: r2d2::Pool<PostgresConnectionManager>,
    pred_by_name: RefCell<HashMap<String, Predicate>>,
    insert_by_name: RefCell<HashMap<String, String>>,
    named_types: RefCell<HashMap<String, Type>>,
}

impl PgDB {
    /// Create a new PgDB object by passing in a Postgres connection string
    // At the moment, persistence with custom types will result in failures
    // on a reconnect, so use a fresh database every time.
    // There's not a good way to persist custom types, so that fix will likely
    // come with optional parameters to seed types in at db startup.
    pub fn new(uri: &str) -> Result<PgDB> {
        // Create database if it doesn't already exist and we can
        let params = try!(uri.into_connect_params().map_err(|_| ErrorKind::UriParse));
        match Connection::connect(params.clone(), ::postgres::TlsMode::None) {
            // Database not found
            Err(ref db_error)
                if db_error.code() == Some(&::postgres::error::UNDEFINED_DATABASE) &&
                       params.database().is_some() => {
                let pg_params = param_into_builder(&params).database("postgres").build(
                    params
                        .host()
                        .clone(),
                );
                let create_query = format!("CREATE DATABASE {}", params.database().unwrap());
                let conn = Connection::connect(pg_params, ::postgres::TlsMode::None)?;
                conn.execute(&create_query, &[])?;
            }
            // If it's not a database not found error, we don't know how to recover, rethrow
            Err(db_error) => Err(db_error)?,
            // The test connection succeeded
            Ok(_) => (),
        }

        // Establish the pool
        let manager = PostgresConnectionManager::new(uri, TlsMode::None)?;
        let pool = r2d2::Pool::new(r2d2::Config::default(), manager)?;
        let conn = pool.get()?;

        // Create schemas
        try!(conn.execute("create schema if not exists facts", &[]));

        // Create Tables
        try!(conn.execute(
            "create table if not exists predicates (id serial primary key, \
                           name varchar not null, \
                           description varchar)",
            &[],
        ));
        try!(conn.execute(
            "create table if not exists fields (\
                           pred_id serial references predicates(id), \
                           ordinal int4 not null, \
                           type varchar not null, \
                           name varchar, \
                           description varchar)",
            &[],
        ));
        try!(conn.execute("create sequence if not exists fact_id", &[]));

        // Make array_to_string immutable to legalize index shenanigans
        // array_to_string is not actually immutable for some arrays (namely when ::text for the
        // element type is not immutable) so this is kind of taking off the safety rails
        try!(conn.execute(
            "alter function array_to_string(anyarray, text) IMMUTABLE",
            &[],
        ));

        // Create incremental PgDB object
        let db = PgDB {
            conn_pool: pool,
            pred_by_name: RefCell::new(HashMap::new()),
            insert_by_name: RefCell::new(HashMap::new()),
            named_types: RefCell::new(
                types::default_types()
                    .iter()
                    .filter_map(|type_| {
                        type_.name().map(|name| (name.to_owned(), type_.clone()))
                    })
                    .collect(),
            ),
        };

        try!(db.rebuild_predicate_cache());

        Ok(db)
    }

    /// Take a connection from the pool, if available
    pub fn conn(&self) -> Result<r2d2::PooledConnection<PostgresConnectionManager>> {
        Ok(self.conn_pool.get()?)
    }

    /// Kick everyone off the database and destroy the data at the provided URI
    pub fn destroy(uri: &str) -> Result<()> {
        let params = try!(uri.into_connect_params().map_err(|_| ErrorKind::UriParse));
        let old_db = try!(params.database().ok_or_else(|| {
            ErrorKind::Arg(format!("No database specified to destroy in {}.", uri))
        }));
        let pg_params = param_into_builder(&params).database("postgres").build(
            params
                .host()
                .clone(),
        );
        let conn = Connection::connect(pg_params, postgres::TlsMode::None)?;
        let disco_query = format!(
            "SELECT pg_terminate_backend(pg_stat_activity.pid) FROM \
                                   pg_stat_activity WHERE pg_stat_activity.datname = '{}' AND \
                                   pid <> pg_backend_pid()",
            &old_db
        );
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
            let conn = self.conn_pool.get()?;
            // Scoped borrow of connection
            let pred_stmt = conn.prepare(
                "select predicates.name,
                              predicates.description, \
                              fields.name, \
                              fields.description, \
                              fields.type from predicates JOIN fields ON \
                              predicates.id = fields.pred_id ORDER BY predicates.id, \
                              fields.ordinal",
            )?;
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
        self.pred_by_name
            .borrow()
            .values()
            .inspect(|pred| self.gen_insert_stmt(pred))
            .count();
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
        let stmt = format!(
            "insert into facts.{} values (DEFAULT, {}) ON \
                            CONFLICT DO NOTHING RETURNING id",
            pred.name,
            args.join(", ")
        );
        self.insert_by_name.borrow_mut().insert(
            pred.name.clone(),
            stmt,
        );
    }

    // Persist a predicate into the database
    // This function is internal because it does not add it to the object, it
    // _only_ puts record of the predicate into the database.
    fn insert_predicate(&self, pred: &Predicate) -> Result<()> {
        let &Predicate {
            ref name,
            ref description,
            ref fields,
        } = pred;
        let conn = self.conn_pool.get()?;
        let stmt = conn.prepare(
            "insert into predicates (name, description) values ($1, $2) returning id",
        )?;
        let pred_id: i32 = stmt.query(&[name, description])?.get(0).get(0);
        for (ordinal, field) in fields.iter().enumerate() {
            try!(conn.execute(
                "insert into fields (pred_id, name, description, type, ordinal) \
                          values ($1, $2, $3, $4, $5)",
                &[
                    &pred_id,
                    &field.name,
                    &field.description,
                    &field.type_.name().ok_or(ErrorKind::Arg(
                        "Field type had no name".to_string(),
                    ))?,
                    &(ordinal as i32),
                ],
            ));
        }
        let table_str = fields
            .iter()
            .map(|field| field.type_.repr())
            .enumerate()
            .map(|(ord, repr)| format!("arg{} {}", ord, repr))
            .collect::<Vec<_>>()
            .join(", ");
        let col_str = fields
            .iter()
            .map(|field| {
                (field.type_.large(), field.type_.repr().contains("[]"))
            })
            .enumerate()
            .map(|(ord, (large, is_array))| if large {
                if is_array {
                    format!("md5(array_to_string(arg{}, ','))", ord)
                } else {
                    format!("md5(arg{}::text)", ord)
                }
            } else {
                format!("arg{}", ord)
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.conn_pool.get()?.execute(
            &format!(
                "create table facts.{} (id INT8 DEFAULT nextval('fact_id') NOT \
                               NULL primary key, {})",
                name,
                table_str
            ),
            &[],
        )?;
        if col_str != "" {
            self.conn_pool.get()?.execute(
                &format!(
                    "create unique index on facts.{} ({})",
                    name,
                    col_str
                ),
                &[],
            )?;
        }
        Ok(())
    }
    /// Adds a new fact to the database, returning false if the fact was already
    /// present in the database, and true if it was inserted.
    pub fn insert_fact(&self, fact: &Fact) -> Result<Option<FactId>> {
        let stmt_str = try!(
            self.insert_by_name
                .borrow()
                .get(&fact.pred_name)
                .ok_or_else(|| {
                    ErrorKind::Internal("Insert Statement Missing".to_string())
                })
        ).clone();
        let conn = self.conn()?;
        let stmt = conn.prepare_cached(&stmt_str)?;


        let out = try!(
            stmt.query(&fact.args
                .iter()
                .flat_map(|x| x.to_sql().into_iter())
                .collect::<Vec<_>>())
        );

        Ok(out.iter().next().map(|x| x.get(0)))
    }

    /// Registers a new type with the database.
    /// This is unstable, and will likely need to be moved to the initialization
    /// of the database object in order to allow reconnecting to an existing
    /// database.
    pub fn add_type(&self, type_: Type) -> Result<()> {
        let name = type_.name().ok_or(ErrorKind::Arg(
            "Tried to add a type with no name".to_string(),
        ))?;
        if !self.named_types.borrow().contains_key(name) {
            self.named_types.borrow_mut().insert(
                name.to_owned(),
                type_.clone(),
            );
            self.rebuild_predicate_cache()
        } else {
            bail!(ErrorKind::Type(format!("{} already registered", name)))
        }
    }

    /// Looks for a named type in the database's registry.
    /// This function is primarily useful for the DSL shorthand for constructing
    /// queries, since it allows you to use names of types when declaring
    /// functions rather than type objects.
    pub fn get_type(&self, type_str: &str) -> Option<Type> {
        self.named_types.borrow().get(type_str).map(|x| x.clone())
    }

    /// Fetches a predicate by name
    pub fn get_predicate(&self, pred_name: &str) -> Option<Predicate> {
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
    pub fn new_predicate(&self, pred: &Predicate) -> Result<()> {
        // The predicate name is used as a table name, check it for legality
        if !valid_name(&pred.name) {
            bail!(ErrorKind::Arg(
                "Invalid name: Use lowercase and \
                                 underscores only"
                    .to_string(),
            ));
        }
        // If this predicate was already registered, check for a match
        match self.pred_by_name.borrow().get(&pred.name) {
            Some(existing) => {
                if existing != pred {
                    bail!(ErrorKind::Arg(format!(
                        "Predicate {} already registered at a \
                                                  different type.\nExisting: {:?}\nNew: {:?}",
                        &pred.name,
                        existing,
                        pred
                    )));
                } else {
                    return Ok(());
                }
            }
            None => (),
        }

        try!(self.insert_predicate(&pred));
        self.gen_insert_stmt(&pred);
        self.pred_by_name.borrow_mut().insert(
            pred.name.clone(),
            pred.clone(),
        );
        Ok(())
    }

    /// Attempt to match the right hand side of a datalog rule against the
    /// database, returning a list of solution assignments to the bound
    /// variables.
    pub fn search_facts<'a>(
        &self,
        query: &Vec<Clause>,
        min_fact_id: Option<FactId>,
    ) -> Result<Vec<(Vec<FactId>, Vec<Value>)>> {
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
                        bail!(ErrorKind::Arg(format!(
                            "{} is not a registered predicate.",
                            clause.pred_name
                        )))
                    }
                };
                for (n, binding) in clause.args.iter().enumerate() {
                    match *binding {
                        MatchExpr::Unbound |
                        MatchExpr::Const(_) => (),
                        MatchExpr::Var(v) => {
                            let v = v as usize;
                            let type_ = pred.fields[n].type_.clone();
                            if v == var_type.len() {
                                var_type.push(type_)
                            } else if v > var_type.len() {
                                if var_type.len() == 0 {
                                    bail!(ErrorKind::Arg(format!(
                                                "First variable not Var(0), got Var({})", v)));
                                }
                                bail!(ErrorKind::Arg(format!(
                                    "Hole between {} and {} in \
                                                              variable numbering.",
                                    var_type.len() - 1,
                                    v
                                )));
                            } else if &var_type[v] != &type_ {
                                bail!(ErrorKind::Arg(format!(
                                    "Variable {} attempt to unify \
                                                              incompatible types {:?} and {:?}",
                                    v,
                                    var_type[v],
                                    type_
                                )));
                            }
                        }
                    }
                }
            }
        }

        // Actually build and execute the query
        let mut tables = Vec::new(); // Predicate names involved in the query,
        // in the sequence they appear
        let mut restricts = vec![format!("1 = 1")]; // Unification expressions, indexed by
        // which join they belong on.
        let mut var_names = Vec::new(); // Translation of variable numbers to
        // sql exprs
        let mut fact_ids = Vec::new(); // Translation of fact ids to sql exprs
        let mut var_types = Vec::new(); // Translation of variable numbers to
        // Types
        let mut vals: Vec<Value> = Vec::new(); // Values to be quoted into the
        // prepared statement
        let mut param_num = 1;

        for (idxc, clause) in query.iter().enumerate() {
            // The clause refers to a table named by the predicate
            let table_name = format!("facts.{}", clause.pred_name);
            // We will refer to it by a numbered alias, to make joining easier
            let alias_name = format!("t{}", idxc);
            let pred = self.pred_by_name
                .borrow()
                .get(&clause.pred_name)
                .unwrap()
                .clone();
            fact_ids.push(format!("{}.id", alias_name));
            let mut clause_elements = Vec::new();
            for (n, arg) in clause.args.iter().enumerate() {
                let proj_str = format!("{}.arg{}", alias_name, n);
                match *arg {
                    MatchExpr::Unbound => (),
                    MatchExpr::Var(var) => {
                        if var >= var_names.len() {
                            // This situation means it's the first occurrence of the variable
                            // We record this definition as the canonical definition for use
                            // in the select, and store the type to know how to extract it.
                            var_names.push(proj_str);
                            let type_ = pred.fields[n].type_.clone();
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
                        vals.push(val.clone());
                        restricts.push(format!("{} = ${}", proj_str, param_num));
                        param_num += 1;
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
        let cache_clause = min_fact_id.map(|fid| {
            (
                format!(
                    "({})",
                    query
                        .iter()
                        .enumerate()
                        .map(|(n, _)| format!("${} <= t{}.id", param_num, n))
                        .collect::<Vec<_>>()
                        .join(" OR ")
                ),
                fid,
            )
        });
        match cache_clause {
            Some((clause, fid)) => {
                use pg::dyn::values::ToValue;
                restricts.push(clause);
                vals.push((fid as u64).to_value())
            }
            _ => (),
        }
        tables.reverse();
        restricts.reverse();
        let main_table = tables.pop().ok_or(ErrorKind::Internal(
            format!("Match clause accesses no tables"),
        ))?;
        let join_query = tables
            .iter()
            .map(|table| format!("JOIN {} ON true", table))
            .collect::<Vec<_>>()
            .join(" ");
        let where_clause = format!("WHERE {}", restricts.join(" AND "));
        let raw_stmt = format!(
            "SELECT {} FROM {} {} {}",
            vars,
            main_table,
            join_query,
            where_clause
        );
        trace!("search_facts: {}", raw_stmt);
        let conn = self.conn()?;
        let stmt = conn.prepare_cached(&raw_stmt)?;
        let sql_vals: Vec<_> = vals.iter().flat_map(|x| x.to_sql()).collect();
        let rows = stmt.query(&sql_vals)?;
        let mut out = Vec::new();
        for row in rows.iter() {
          let mut row_iter = RowIter::new(&row);
          let mut ids = Vec::new();
          for _ in 0..fact_ids.len() {
              match row_iter.next() {
                  Some(e) => ids.push(e),
                  None => panic!("Failure loading fact ids from row"),
              }
          }
          let mut vars = Vec::new();
          for var_type in var_types.iter() {
              match var_type.extract(&mut row_iter) {
                  Some(e) => vars.push(e),
                  None => panic!("Failure loading var from row"),
              }
          }
          out.push((ids, vars));
        }
        Ok(out)
    }
}

fn valid_name(name: &String) -> bool {
    name.chars().all(|ch| match ch {
        'a'...'z' | '_' => true,
        _ => false,
    })
}
