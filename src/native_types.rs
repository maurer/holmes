use db_types::values::Value;
use db_types::types::Type;
use std::sync::Arc;
pub type PredId = u64;

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Predicate {
  pub name  : String,
  pub types : Vec<Arc<Type>>
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Fact {
  pub pred_name : String,
  pub args : Vec<Arc<Value>>
}

pub type HVar = usize;

#[derive(Clone,Debug,Hash,Eq)]
pub enum MatchExpr {
  Unbound,
  Var(HVar),
  Const(Arc<Value>)
}
pub use native_types::MatchExpr::*;

impl PartialEq for MatchExpr {
  fn eq(&self, other : &MatchExpr) -> bool {
    match (self, other) {
      (&Unbound, &Unbound) => true,
      (&Var(x), &Var(y)) => x == y,
      (&Const(ref v), &Const(ref vv)) => v == vv,
      _ => false
    }
  }
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub enum BindExpr {
  Normal(MatchExpr),
  Destructure(Vec<BindExpr>),
  Iterate(Box<BindExpr>)
}
pub use native_types::BindExpr::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Clause {
  pub pred_name : String,
  pub args : Vec<MatchExpr>
}

#[derive(Clone,Debug,Hash,Eq)]
pub enum Expr {
  EVar(HVar),
  EVal(Arc<Value>),
  EApp(String, Vec<Expr>)
}
pub use native_types::Expr::*;

impl PartialEq for Expr {
   fn eq(&self, other : &Expr) -> bool {
     match (self, other) {
       (&EVar(ref x), &EVar(ref y)) => x == y,
       (&EVal(ref x), &EVal(ref y)) => x == y,
       (&EApp(ref s0, ref ex0), &EApp(ref s1, ref ex1)) => (s0 == s1) && (ex0 == ex1),
       _ => false
     }
   }
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Rule {
  pub head  : Clause,
  pub body  : Vec<Clause>,
  pub wheres : Vec<WhereClause>
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct WhereClause {
  pub lhs : BindExpr,
  pub rhs : Expr
}

pub struct HFunc {
  pub input_type   : Arc<Type>,
  pub output_type  : Arc<Type>,
  pub run : Box<Fn(Arc<Value>) -> Arc<Value>>
}
