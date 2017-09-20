//! Holmes Language Types
//!
//! The types defined in this module are used to define the parts of the Holmes
//! language itself, and are used for writing rules, facts, etc.
use pg::dyn::{Type, Value};

/// A `Predicate` is a name combined with a list of typed slots, e.g.
///
/// ```c
/// foo(uint64, string)
/// ```
///
/// would be represented as
///
/// ```
/// use holmes::pg::dyn::types;
/// use holmes::engine::types::{Predicate, Field};
/// use std::sync::Arc;
/// Predicate {
///     name: "foo".to_string(),
///     description: None,
///     fields: vec![Field {
///         name: None,
///         description: None,
///         type_: Arc::new(types::UInt64)
///     }, Field {
///         name: None,
///         description: None,
///         type_: Arc::new(types::String)
///     }]
/// };
/// ```
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub struct Predicate {
    /// Predicate Name
    pub name: String,
    /// Description of what it means for this predicate to be true.
    /// Purely documentation, not mechanical
    pub description: Option<String>,
    /// Predicate fields
    pub fields: Vec<Field>,
}

/// Field for use in a predicate
/// The name is for use in selective matching or unordered definition,
/// and the description is to improve readability of code and comprehension of
/// results.
/// The `Type` is the only required component of a field, as it defines how to
/// actually interact with the field.
#[derive(Clone, Debug, Hash, Eq)]
pub struct Field {
    /// Name of field, for use in matching and instantiating predicates
    pub name: Option<String>,
    /// Description of the field, purely documentation, not mechanical
    pub description: Option<String>,
    /// Type of the predicate, explaining how to store and retrieve
    /// information from the `FactDB`
    pub type_: Type,
}

// Manually implement PartialEq to work around rustc #[derive(PartialEq)] bug
// https://github.com/rust-lang/rust/issues/39128
impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        (self.name == other.name) && (self.description == other.description) &&
            (self.type_.eq(&other.type_))
    }
}

/// A `Fact` is a particular filling of a `Predicate`'s slots such that it is
/// considered true.
///
/// Following the `Predicate` example,
///
/// ```c
/// foo(3, "argblarg")
/// ```
///
/// would be constructed as
///
/// ```
/// use holmes::pg::dyn::values::ToValue;
/// use holmes::engine::types::Fact;
/// Fact {
///   pred_name : "foo".to_string(),
///   args : vec![3.to_value(), "argblarg".to_value()]
/// };
/// ```
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub struct Fact {
    /// Predicate name
    pub pred_name: String,
    /// Slot values which make the predicate true
    pub args: Vec<Value>,
}

/// `Var` is placeholder type for the representation of a variable in the
/// Holmes langauge. At the moment, it is just an index, and so is
/// transparently an integer, but this behavior should not be relied upon, as
/// it is likely that in the future it will carry other information (name,
/// type, etc.) for improved debugging.
pub type Var = usize;

/// A `MatchExpr` represents the possible things that could show up in a slot
/// in the body of a rule
#[derive(Clone, Debug, Hash, Eq)]
pub enum MatchExpr {
    /// We do not care about the contents of the slot
    Unbound,
    /// Bind the contents of the slot to this variable if undefined, otherwise
    /// only match if the definition matches the contents of the slot
    Var(Var),
    /// Only match if the contents of the slot match the provided value
    Const(Value),
}

// This is a temporary impl. PartialEq should be derivable, but a compiler bug
// is preventing it from being derived
impl PartialEq for MatchExpr {
    fn eq(&self, other: &MatchExpr) -> bool {
        use self::MatchExpr::*;
        match (self, other) {
            (&Unbound, &Unbound) => true,
            (&Var(x), &Var(y)) => x == y,
            (&Const(ref v), &Const(ref vv)) => v == vv,
            _ => false,
        }
    }
}

/// A `BindExpr` is what appears on the left hand of the assignment in a
/// Holmes rule where clause.
/// It describes how to extend or limit the answer set based on the value
/// on the right side.
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub enum BindExpr {
    /// Use the same filtering/binding rules as in a match expression
    Normal(MatchExpr),
    /// Treat the value as a tuple, and run each inner bind expression on the
    /// corresponding tuple element
    Destructure(Vec<BindExpr>),
    /// Treat the value as a list, and extend the answer set with a new
    /// possibility for each element in the list, binding to each list element
    /// with the provided `BindExpr`
    /// This is simlar the list monadic bind.
    Iterate(Box<BindExpr>),
}

/// A `Clause` to be matched against, as you would see in the body of a datalog
/// rule.
///
/// Continuing with our running example,
///
/// ```c
/// foo(_, x)
/// ```
///
/// (match all `foo`s, bind the second slot to x) would be constructed as
///
/// ```
/// use holmes::engine::types::{Clause,MatchExpr};
/// Clause {
///   pred_name : "foo".to_string(),
///   args : vec![MatchExpr::Unbound, MatchExpr::Var(0)]
/// };
/// ```
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub struct Clause {
    /// Name of the predicate to match against
    pub pred_name: String,
    /// List of how to restrict or bind each slot
    pub args: Vec<MatchExpr>,
}

/// `Expr` represents the right hand side of the where clause sublanguage of
/// Holmes.
#[derive(Clone, Debug, Hash, Eq)]
pub enum Expr {
    /// Evaluates to whatever the inner variable is defined to.
    Var(Var),
    /// Evaluates to the value provided directly.
    Val(Value),
    /// Applies the function in the registry named the first argument to the
    /// list of arguments provided as the second
    App(String, Vec<Expr>),
}

// As per prvious, this is only needed due to a compiler bug. In the long
// run this impl should be derived
impl PartialEq for Expr {
    fn eq(&self, other: &Expr) -> bool {
        use self::Expr::*;
        match (self, other) {
            (&Var(ref x), &Var(ref y)) => x == y,
            (&Val(ref x), &Val(ref y)) => x == y,
            (&App(ref s0, ref ex0), &App(ref s1, ref ex1)) => (s0 == s1) && (ex0 == ex1),
            _ => false,
        }
    }
}

/// A `Rule` represents a complete inference technique in the Holmes system
/// If the `body` clauses match, the `wheres` clauses are run on the answer
/// set, producing a new answer set, and the `head` clause is instantiated
/// at that answer set and inserted into the database.
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub struct Rule {
    /// Identifier for the rule
    pub name: String,
    /// Template for the facts this rule will output
    pub head: Clause,
    /// Datalog body to search the database with
    pub body: Vec<Clause>,
    /// Embedded language to call native functions on the results
    pub wheres: Vec<WhereClause>,
}

/// A `WhereClause` is a single assignment in the Holmes sublanguage.
/// The right hand side is evaluated, and bound to the left hand side,
/// producing a new answer set.
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub struct WhereClause {
    /// Instructions on how to assign the evaluated rhs
    pub lhs: BindExpr,
    /// The expression to evaluate
    pub rhs: Expr,
}

/// A `Func` is the wrapper around dynamically typed functions which may be
/// registered with the engine to provide extralogical functionality.
pub struct Func {
    /// The type of the `Value` the function expects to receive as input
    pub input_type: Type,
    /// The type of the `Value` the function will produce as output
    pub output_type: Type,
    /// The function itself
    pub run: Box<Fn(Value) -> Value>,
}
