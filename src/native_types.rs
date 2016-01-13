use std::str::FromStr;
use std::string::{ToString, String};
use std::borrow::ToOwned;

pub type PredId = u64;

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub enum HType {
  UInt64,
  HString,
  Blob,
  List(Box<HType>)
}
pub use native_types::HType::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq,RustcDecodable,RustcEncodable)]
pub enum HValue {
  UInt64V(u64),
  HStringV(String),
  BlobV(Vec<u8>),
  ListV(Vec<HValue>)
}
pub use native_types::HValue::*;

pub trait ToHValue {
  fn to_hvalue(self) -> HValue;
}

impl ToHValue for String {
  fn to_hvalue(self) -> HValue {
    HStringV(self)
  }
}

impl<'a> ToHValue for &'a str {
  fn to_hvalue(self) -> HValue {
    HStringV(self.to_string())
  }
}

impl ToHValue for u64 {
  fn to_hvalue(self) -> HValue {
    UInt64V(self)
  }
}

impl ToHValue for Vec<u8> {
  fn to_hvalue(self) -> HValue {
    BlobV(self)
  }
}

pub fn type_check(vty : (&HValue, &HType)) -> bool {
  match vty {
      (&UInt64V(_),  &UInt64)
    | (&HStringV(_), &HString)
    | (&BlobV(_),    &Blob) => true,
    _ => false
  }
}

impl FromStr for HType {
  type Err = String;
  fn from_str(s : &str) -> Result<HType, String> {
    match s {
      "uint64" => Ok(UInt64),
      "string" => Ok(HString),
      "blob"   => Ok(Blob),
      _ => Err(s.to_owned())
    }
  }
}

impl ToString for HType {
  fn to_string(&self) -> String {
    match *self {
      UInt64       => "uint64".to_string(),
      HString      => "string".to_string(),
      Blob         => "blob".to_string(),
      List(ref ty) => format!("[{}]", ty.to_string())
    }
  }
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Predicate {
  pub name  : String,
  pub types : Vec<HType>
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Fact {
  pub pred_name : String,
  pub args : Vec<HValue>
}

pub type HVar = u32;

#[derive(PartialEq,Clone,Debug,Hash,Eq,RustcDecodable,RustcEncodable)]
pub enum MatchExpr {
  Unbound,
  Var(HVar),
  HConst(HValue)
}
pub use native_types::MatchExpr::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq,RustcDecodable,RustcEncodable)]
pub enum BindExpr {
  Normal(MatchExpr),
  Iterate(MatchExpr)
}
pub use native_types::BindExpr::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq,RustcDecodable,RustcEncodable)]
pub struct Clause {
  pub pred_name : String,
  pub args : Vec<MatchExpr>
}

#[derive(PartialEq,Clone,Debug,Hash,Eq,RustcDecodable,RustcEncodable)]
pub enum Expr {
  EVar(HVar),
  EVal(HValue),
  EApp(String, Vec<Expr>)
}
pub use native_types::Expr::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq,RustcDecodable,RustcEncodable)]
pub struct Rule {
  pub head  : Clause,
  pub body  : Vec<Clause>,
  pub wheres : Vec<WhereClause>
}

#[derive(PartialEq,Clone,Debug,Hash,Eq,RustcDecodable,RustcEncodable)]
pub struct WhereClause {
  pub asgns : Vec<BindExpr>,
  pub rhs : Expr
}

pub struct HFunc {
  pub input_types  : Vec<HType>,
  pub output_types : Vec<HType>,
  pub run : Box<Fn(Vec<HValue>) -> Vec<HValue>>
}
