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
//! In a rule or a query (in the next section), the possible restrictions on
//! each slot are:
//!
//!   * Unbound: `[_]`
//!   * Constant Equality: `(value)`
//!   * Variable unification `var`
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
//!
//! let mut res = try!(query!(holmes, distance(A, [_], [_])));
//!
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
//! ## Extensions
//!
//! While Datalog itself is interesting, writing yet-another-datalog engine
//! is not the goal of this project. Next, we'll go over some of the new
//! features of this system.
//!
//! ### Functions
//! Normally, logic languages expect the computation to be encoded as rules
//! only (or in special cases, as external predicates). In order to allow
//! the user to write things which make more sense as traditional code, we
//! allow the binding of functions:
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
//! # holmes_exec!(holmes, {
//! func!(let f : uint64 -> uint64 = |x : &u64| {
//!   x * 3
//! })
//! # });
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! In this case, we have declared a function called `f`, said that it takes
//! as input a `uint64`, and should output a `uint64`.
//! The type of the input to the function should be the output of the `.get()`
//! call of the relevant value, which will usually be a reference to the rust
//! equivalent of the type.
//! The output should be a value which `.to_value()` will convert to a
//! correctly typed `Value`.
//!
//! Additionally, the type system allows for tuples and lists. Tuple types
//! are denoted `(t1, t2)`, and list types are denoted `[t]`. Lists and tuples
//! will be unpacked through by the `func!` macro, so a function with a
//! `[uint64]` input would expect to take a `Vec<&u64>`, and a function taking
//! `(string, uint64)` would expect to take a (&String, &u64).
//! For example:
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
//! # holmes_exec!(holmes, {
//! func!(let replicate : (string, uint64) -> [string] =
//!   |(s, n) : (&String, &u64)| {
//!     let mut vec : Vec<String> = Vec::new();
//!     for i in 0..*n {
//!       vec.push(s.clone());
//!     };
//!     vec
//!   }
//! )
//! # });
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! ## Where Clauses
//!
//! Telling Holmes about functions isn't useful without a way to use them.
//! Where clauses are a way to perform a transformation on the data after the
//! map, but before the head clause is produced and sent to the database.
//!
//! Extending the example from earlier, we might want to generate a distances
//! for the connection paths we found.
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
//! //rule!(distance(B, A, N) <= distance(A, B, N));
//!   predicate!(path(string, string, uint64));
//!   rule!(path(A, B, N) <= distance(A, B, N));
//!   func!(let add : (uint64, uint64) -> uint64 = |(x, y) : (&u64, &u64)| {
//!     x + y
//!   });
//!   rule!(path(A, C, NSum) <= path(A, B, N1) & path(B, C, N2), {
//!     let NSum = {add([N1], [N2])}
//!   })
//! });
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! The astute reader will notice there is something wrong with this example.
//! It builds, and it runs, I'm not trying to mess with you while teaching.
//! However, the last rule we added (which does the sum of the distances) will
//! loop forever if there is any cycle in the `distance` predicate.
//! This is why I commented out the rule flipping the distance direction
//! around, as this would cause this example to run infinitely.
//!
//! Normally in Datalog, we have a termination property - no matter what
//! rules or facts you add, the database will always eventually stop growing.
//! This proof follows from the inability of a rule firing to introduce a new
//! value, which means there are only a finite number of derivable facts. With
//! the addition of where clauses, we lose this property, because new values
//! can appear, as per the `add` function above.
//!
//! However, we also add other kinds of binds to the where clause that
//! can help the programmer control this kind of situation.
//!
//!
//! N.B. the postgres backend doesn't currently support list persistence, so
//! if you wanted to use a list in a predicate, you'd actually need to make a
//! custom `Path` type and value that knew how to store itself, perhaps via
//! `postgres-array`
//!
//! ### Binds
//!
//! #### Variable binding
//! This is as in the inital example. They are written `let x = expression`,
//! and simply bind the expression to the variable.
//!
//! #### Destructuring
//! This kind of bind is basically just shorthand to prevent the need for
//! functions like `access_tuple_field_1`, `access_tuple_field_2`.
//! It is written `let (x, y, z) = expression`
//!
//! #### Value binding
//! This is the first unusual kind of binding, and the one we can use to fix
//! up the previous example. Value binds are written `let (expr) = expr2`.
//! If `expr` and `expr2` evaluate to the same value, this expression has no
//! effect. However, if `expr` and `expr2` differ, the variable assignment
//! currently generated by the where clause will stop.
//!
//! To fix the previous example, we can track the path we've gone through thus
//! far, and store it in an additional slot in the `path` predicate.
//! Then, in the where clause for adding a new step to the path, we can check
//! for membership in the existing path. If it is present, we can use a value
//! binding to stop pursuing this avenue. If it is not present, then we can
//! proceed as before.
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
//! try!(holmes_exec!(holmes, {
//!   predicate!(distance(string, string, uint64));
//!   fact!(distance("New York", "Albuquerque", 1810));
//!   fact!(distance("New York", "Las Vegas", 2225));
//!   fact!(distance("Las Vegas", "Palo Alto", 542));
//!   fact!(distance("Rome", "Florence", 173));
//!   rule!(distance(B, A, N) <= distance(A, B, N));
//!   predicate!(path(string, string, [string], uint64));
//!   func!(let two_vec : (string, string) -> [string] =
//!     |(x, y) : (&String, &String)| { vec![x.clone(), y.clone()] });
//!   rule!(path(A, B, steps, N) <= distance(A, B, N), {
//!     let steps = {two_vec([A], [B])}});
//!   func!(let add : (uint64, uint64) -> uint64 = |(x, y) : (&u64, &u64)| {
//!     x + y
//!   });
//!   func!(let append : (string, [string]) -> [string] =
//!     |(x, y) : (&String, Vec<&String>)| {
//!       let mut out : Vec<String> = y.into_iter().cloned().collect();
//!       out.push(x.clone());
//!       out
//!     });
//!   func!(let mem : (string, [string]) -> bool =
//!     |(needle, haystack) : (&String, Vec<&String>)| {
//!       haystack.contains(&needle)
//!     });
//!   rule!(path(A, C, path2, NSum) <= path(A, B, path, N1)
//!                                  & distance(B, C, N2), {
//!     // If we've already walked over C, we aren't interested
//!     let (false) = {mem([C], [path])};
//!     let path2 = {append([C], [path])};
//!     let NSum = {add([N1], [N2])}
//!   })
//! }));
//! let mut res = query!(holmes, path(("New York"), dest, [_], dist)).unwrap();
//! # res.sort_by(|x, y| x.partial_cmp(y).unwrap_or(
//! #   ::std::cmp::Ordering::Greater));
//!
//! assert_eq!(res,
//!            vec![
//!                 vec!["Albuquerque".to_value(), 1810.to_value()],
//!                 vec!["Las Vegas".to_value(), 2225.to_value()],
//!                 vec!["Palo Alto".to_value(), 2767.to_value()],
//!                ]);
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! #### Iteration
//! The last kind of bind is the iterative bind. This works similarly to the
//! List monad in Haskell if you are familiar with it, but you don't need
//! to know anything about that to proceed.
//!
//! An iterative bind is written `let [x] = expr`, where the expression should
//! evaluate to a list-typed value. When this bind is run, the set of possible
//! answers splits into a different instance for each value in the list. So, if
//! we had
//!
//! ```c
//! rule!(q(x, y) <= p(y), {
//!   let [x] = f(y)
//! })
//! ```
//!
//! it would first find all `y` such that `p(y)`, and then for each of them,
//! it would apply `f` and get a list. Imagine that `f` just returns a list of
//! `y` and `y + 1`, and that `p` is only populated with `p(1)` and `p(2)`.
//!
//! The match would produce the possible assignment sets `y = 1` and `y = 2`.
//! After running the where clause, the first one would become `x = 1, y = 1`,
//! `x = 2, y = 1`, and the secould would become `x = 2, y = 2`, `x = 3, y = 2`
//! . This ends with the database containing `q(1, 1), q(2, 1), q(2, 2),
//! q(3, 2)`.
//!
//! That example is somewhat abstract, but hopefully it illustrates the
//! multiplicative effect of the iteration bind. The iteration bind can also
//! be used to terminate early a rule, similar to the value bind, by iterating
//! over an empty list. If an iteration bind is used multiple times in a where
//! clause, it will operate on each of the new answer sets from the previous
//! iteration bind individually.
//!
//! As a more concrete example, say we wanted to define a predicate
//! which contained all sities that might be used on a path from New York to
//! Palo Alto. We can take the example from earlier and add:
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
//! # try!(holmes_exec!(holmes, {
//! #   predicate!(distance(string, string, uint64));
//! #   fact!(distance("New York", "Albuquerque", 1810));
//! #   fact!(distance("New York", "Las Vegas", 2225));
//! #   fact!(distance("Las Vegas", "Palo Alto", 542));
//! #   fact!(distance("Rome", "Florence", 173));
//! #   rule!(distance(B, A, N) <= distance(A, B, N));
//! #   predicate!(path(string, string, [string], uint64));
//! #   func!(let two_vec : (string, string) -> [string] =
//! #     |(x, y) : (&String, &String)| { vec![x.clone(), y.clone()] });
//! #   rule!(path(A, B, steps, N) <= distance(A, B, N), {
//! #     let steps = {two_vec([A], [B])}});
//! #   func!(let add : (uint64, uint64) -> uint64 = |(x, y) : (&u64, &u64)| {
//! #     x + y
//! #   });
//! #   func!(let append : (string, [string]) -> [string] =
//! #     |(x, y) : (&String, Vec<&String>)| {
//! #       let mut out : Vec<String> = y.into_iter().cloned().collect();
//! #       out.push(x.clone());
//! #       out
//! #     });
//! #   func!(let mem : (string, [string]) -> bool =
//! #     |(needle, haystack) : (&String, Vec<&String>)| {
//! #       haystack.contains(&needle)
//! #     });
//! #   rule!(path(A, C, path2, NSum) <= path(A, B, path, N1)
//! #                                  & distance(B, C, N2), {
//! #     // If we've already walked over C, we aren't interested
//! #     let (false) = {mem([C], [path])};
//! #     let path2 = {append([C], [path])};
//! #     let NSum = {add([N1], [N2])}
//! #   });
//! predicate!(on_the_road(string, string, string));
//! rule!(on_the_road(A, B, stop) <= path(A, B, path, [_]), {
//!   let [stop] = [path]
//! })
//! # }));
//! let mut res = query!(holmes, on_the_road(("New York"), ("Palo Alto"),
//!                                          stop)).unwrap();
//! # res.sort_by(|x, y| x.partial_cmp(y).unwrap_or(
//! #   ::std::cmp::Ordering::Greater));
//!
//! assert_eq!(res,
//!            vec![
//!                 vec!["Las Vegas".to_value()],
//!                 vec!["New York".to_value()],
//!                 vec!["Palo Alto".to_value()],
//!                ]);
//! # Ok(())
//! # }
//! # fn main () {f().unwrap()}
//! ```
//!
//! # Caveats
//!
//! * If you use custom types, you cannot currently reconnect to the database.
//!   This will be fixed in the near future.
//! * Lists cannot be persisted in the postgres backend. If you must have a
//!   list persisted, create a custom ListOfExtendedType type and Value.
//!   Note that these will not be able to be used with an iteration bind,
//!   so if you must do that, you will need to convert between them with a
//!   function first.
//!   You may find postgres-array useful for writing your type.
#![warn(missing_docs)]
extern crate postgres;
extern crate postgres_array;
extern crate rustc_serialize;
#[macro_use]
extern crate log;

pub mod pg;
pub mod fact_db;
pub mod mem_db;
pub mod engine;
pub mod edsl;

use pg::dyn::{Value, Type};
use fact_db::FactDB;
use mem_db::MemDB;

/// Defines the database connection parameters
#[derive (Clone)]
pub enum DB {
    /// A postgres database, via a connection string
    Postgres(String),
    /// A memory backed database, *VERY INEFFICIENT*
    Memory,
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
    Engine(engine::Error),
}

use engine::Engine;
use pg::PgDB;

/// `Result` is a shorthand type for the standard `Result` specialized to our
/// `Error type.
pub type Result<T> = ::std::result::Result<T, Error>;

use std::fmt::{Debug, Display, Formatter};

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        use self::Error::*;
        match *self {
            NoDB => fmt.write_str("No database specified"),
            PgConnect(ref e) => fmt.write_fmt(format_args!("Connection failed: {}", e)),
            Pg(ref e) => fmt.write_fmt(format_args!("Postgres error: {}", e)),
            Db(ref e) => fmt.write_fmt(format_args!("Deductive DB error: {}", e)),
            PgConnectStr(ref e) => {
                fmt.write_fmt(format_args!("Connection string failed to \
                                            parse: {}",
                                           e))
            }
            IO(ref e) => fmt.write_fmt(format_args!("IO failed: {}", e)),
            Engine(ref e) => fmt.write_fmt(format_args!("Engine Error: {}", e)),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        Display::fmt(self, fmt)
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        use self::Error::*;
        match self {
            &NoDB => "No database specified",
            &PgConnect(ref e) => e.description(),
            &Pg(ref e) => e.description(),
            &Db(ref e) => e.description(),
            &IO(ref e) => e.description(),
            &Engine(ref e) => e.description(),
            &PgConnectStr(_) => "Connection string failed to parse",
        }
    }
}

impl From<::postgres::error::ConnectError> for Error {
    fn from(ce: ::postgres::error::ConnectError) -> Error {
        Error::PgConnect(ce)
    }
}

impl From<::postgres::error::Error> for Error {
    fn from(e: ::postgres::error::Error) -> Error {
        Error::Pg(e)
    }
}

impl From<Box<::std::error::Error>> for Error {
    fn from(e: Box<::std::error::Error>) -> Error {
        Error::Db(e)
    }
}

impl From<engine::Error> for Error {
    fn from(e: engine::Error) -> Error {
        Error::Engine(e)
    }
}

impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Error {
        Error::IO(e)
    }
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
                let disco_query = format!("SELECT pg_terminate_backend(pg_stat_activity.\
                                           pid) FROM pg_stat_activity WHERE \
                                           pg_stat_activity.datname = '{}' \
                                           AND pid <> pg_backend_pid()",
                                          &old_db);
                try!(conn.execute(&disco_query, &[]));
                let drop_query = format!("DROP DATABASE {}", &old_db);
                try!(conn.execute(&drop_query, &[]));
            }
            &DB::Memory => (),
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
                // TODO accept success or db already exists, not other errors
                let _ = conn.execute(&create_query, &[]);
                Ok(Box::new(try!(PgDB::new(str))))
            }
            &DB::Memory => Ok(Box::new(MemDB::new())),
        }
    }
}

/// Encapsulates the user-level interface to a holmes database
pub struct Holmes {
    db: DB,
    engine: Engine,
}

impl Holmes {
    /// Create a new holmes instance from a db specification
    pub fn new(db: DB) -> Result<Holmes> {
        Ok(Holmes {
            engine: Engine::new(try!(db.create())),
            db: db,
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
    pub fn add_predicate(&mut self, pred: &engine::types::Predicate) -> Result<()> {
        self.engine.new_predicate(pred).map_err(|e| Error::Engine(e))
    }

    /// Add a new fact to the database. The predicate must be registered first.
    /// Duplicate facts are ignored.
    pub fn add_fact(&mut self, fact: &engine::types::Fact) -> Result<()> {
        self.engine.new_fact(fact).map_err(|e| Error::Engine(e))
    }

    /// Query the database with the right hand side of a datalog rule, returning
    /// all legal assignments to the variables to make it true
    pub fn query(&mut self, query: &Vec<engine::types::Clause>) -> Result<Vec<Vec<Value>>> {
        self.engine.derive(query).map_err(|e| Error::Engine(e))
    }

    /// Get a dynamic type registered with Holmes by name. If it is present, it
    /// returns `Some(Type)`, otherwise it returns `None`. The type database is
    /// prepopulated with a few defaults, but otherwise this will only work if
    /// you registered the type via `add_type`.
    pub fn get_type(&self, name: &str) -> Option<Type> {
        self.engine.get_type(name)
    }

    /// Registers a new type with Holmes. It must be a named type, e.g.
    /// `type.name()` would return the `Some` branch.
    /// Types should be registered exactly once
    pub fn add_type(&mut self, type_: Type) -> Result<()> {
        self.engine.add_type(type_).map_err(|e| Error::Engine(e))
    }

    /// Adds a new inference rule to the Holmes program
    pub fn add_rule(&mut self, rule: &engine::types::Rule) -> Result<()> {
        self.engine.new_rule(rule).map_err(|e| Error::Engine(e))
    }

    /// Registers a dynamically typed function for use in Holmes where clauses
    pub fn reg_func(&mut self,
                    name: String,
                    input_type: Type,
                    output_type: Type,
                    func: Box<Fn(Value) -> Value>)
                    -> Result<()> {
        self.engine.reg_func(name,
                             engine::types::Func {
                                 input_type: input_type,
                                 output_type: output_type,
                                 run: func,
                             });
        Ok(())
    }
}
