use capnp::struct_list;
use holmes_capnp::holmes;

use std::str::FromStr;
use std::string::{ToString, String};
use std::borrow::ToOwned;

use capnp::traits::FromStructReader;

pub type PredId = u64;

#[derive(Copy,PartialEq,Clone,Debug,Hash,Eq)]
pub enum HType {
  UInt64,
  HString,
  Blob
}
use native_types::HType::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub enum HValue {
  UInt64V(u64),
  HStringV(String),
  BlobV(Vec<u8>)
}
use native_types::HValue::*;

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
    (match self {
      &UInt64  => {"uint64"}
      &HString => {"string"}
      &Blob    => {"blob"}
    }).to_string()
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

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub enum MatchExpr {
  Unbound,
  Var(HVar),
  HConst(HValue)
}
use native_types::MatchExpr::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Clause {
  pub pred_name : String,
  pub args : Vec<MatchExpr>
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub enum Expr {
  EVar(HVar),
  EVal(HValue),
  EApp(String, Vec<Expr>)
}
use native_types::Expr::*;

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct Rule {
  pub head  : Clause,
  pub body  : Vec<Clause>,
  pub wheres : Vec<WhereClause>
}

#[derive(PartialEq,Clone,Debug,Hash,Eq)]
pub struct WhereClause {
  pub asgns : Vec<MatchExpr>,
  pub rhs : Expr
}

pub struct HFunc {
  pub input_types  : Vec<HType>,
  pub output_types : Vec<HType>,
  pub run : Box<Fn(Vec<HValue>) -> Vec<HValue> + 'static + Send>
}

pub fn convert_types<'a> (types_reader : struct_list::Reader<'a, holmes::h_type::Reader<'a>>)
   -> Vec<HType> {
  let mut types = Vec::new();
  for type_reader in types_reader.iter() {
    match type_reader.which() {
      Ok(holmes::h_type::Uint64(())) => {types.push(UInt64);}
      Ok(holmes::h_type::String(())) => {types.push(HString);}
      Ok(holmes::h_type::Blob(())) => {types.push(Blob);}
      Err(_) => {panic!("Unknown HType")}
    }
  }
  types
}

pub fn convert_val<'a> (val_reader : holmes::val::Reader<'a>)
  -> HValue {
  match val_reader.which() {
    Ok(holmes::val::Uint64(v)) => UInt64V(v),
    Ok(holmes::val::String(s)) => HStringV(s.unwrap().to_owned()),
    Ok(holmes::val::Blob(b)) => {
      let bv = b.unwrap().to_owned();
      BlobV(bv)
    }
    Err(_) => panic!("Invalid value on wire")
  }
}

pub fn capnp_val<'a> (mut val_builder : holmes::val::Builder<'a>,
                      h_val : &HValue) {
  match h_val {
    &HStringV(ref x) => val_builder.set_string(x),
    &BlobV(ref x)    => val_builder.set_blob(x),
    &UInt64V(x) => val_builder.set_uint64(x)
  }
}

pub fn capnp_type<'a> (mut type_builder : holmes::h_type::Builder<'a>,
                        h_type : &HType) {
  match *h_type {
    HString => type_builder.set_string(()),
    Blob    => type_builder.set_blob(()),
    UInt64  => type_builder.set_uint64(())
  }
}

pub fn convert_vals<'a> (args_reader : struct_list::Reader<'a, holmes::val::Reader<'a>>)
  -> Vec<HValue> {
  let mut args = Vec::new();
  for arg_reader in args_reader.iter() {
    args.push(convert_val(arg_reader));
  }
  args
}

pub fn convert_expr<'a>(expr_reader : holmes::expr::Reader<'a>) -> Expr {
  match expr_reader.which() {
    Ok(holmes::expr::Var(v)) => EVar(v),
    Ok(holmes::expr::Val(val)) => EVal(convert_val(val.unwrap())),
    Ok(holmes::expr::App(f_expr)) => {
      let f_expr = f_expr.unwrap();
      EApp(f_expr.get_func().unwrap().to_owned(),
           convert_many(f_expr.get_args().unwrap(),
                        convert_expr))
    }
    Err(_) => panic!("Unidentified expr branch")
  }
}

pub fn convert_where<'a>(where_reader : holmes::where_clause::Reader<'a>) -> WhereClause {
  WhereClause {
    asgns : convert_many(where_reader.get_lhs().unwrap(),
                         convert_body_expr),
    rhs : unimplemented!()
  }
}

pub fn convert_body_expr<'a>(body_expr_reader : holmes::body_expr::Reader<'a>) -> MatchExpr {
  match body_expr_reader.which() {
    Ok(holmes::body_expr::Unbound(())) => Unbound,
    Ok(holmes::body_expr::Var(v)) => Var(v),
    Ok(holmes::body_expr::Const(val)) =>
      HConst(convert_val(val.unwrap())),
    Err(_) => panic!("Unknown expr type")
  }
}

pub fn convert_clause<'a>(clause_reader : holmes::body_clause::Reader<'a>)
                         -> Clause {
  let pred = clause_reader.get_predicate().unwrap();
  Clause {
    pred_name : pred.to_owned(),
    args : convert_many(clause_reader.get_args().unwrap(),
                        convert_body_expr)
  }
}

pub fn convert_many<'a, T : FromStructReader<'a>, U,
                     F : Fn(T) -> U>(
    reader   : struct_list::Reader<'a, T>,
    conv_one : F) -> Vec<U> {
  reader.iter().map(conv_one).collect()
}

pub fn convert_clauses<'a>(clauses_reader : struct_list::Reader<'a,
                       holmes::body_clause::Reader<'a>>) ->
                       Vec<Clause> {
  convert_many(clauses_reader, convert_clause)
}

pub fn capnp_rule<'a>(mut rule_builder : holmes::rule::Builder<'a>, rule : &Rule) {
  {
    let head_builder = rule_builder.borrow().init_head();
    capnp_clause(head_builder, &rule.head);
  }
  let mut body_builder = rule_builder.borrow().init_body(rule.body.len() as u32);
  for (i, clause) in rule.body.iter().enumerate() {
    capnp_clause(body_builder.borrow().get(i as u32), clause)
  }
}

pub fn convert_rule<'a>(rule_reader : holmes::rule::Reader<'a>) -> Rule {
  Rule {
    head : convert_clause(rule_reader.get_head().unwrap()),
    body : convert_many(rule_reader.get_body().unwrap(), convert_clause),
    wheres : convert_many(rule_reader.get_where().unwrap(),
                          convert_where)
  }
}
pub fn capnp_body_expr<'a>(mut expr_builder : holmes::body_expr::Builder<'a>,
                  expr : &MatchExpr) {
  match expr {
    &Unbound => expr_builder.set_unbound(()),
    &Var(v) => expr_builder.set_var(v),
    &HConst(ref val) => capnp_val(expr_builder.init_const(), val)
  }
}

pub fn capnp_clause<'a>(mut clause_builder : holmes::body_clause::Builder<'a>,
                        clause : &Clause) {
  clause_builder.set_predicate(&clause.pred_name[..]);
  let mut clause_args = clause_builder.init_args(clause.args.len() as u32);
  for (i, arg) in clause.args.iter().enumerate() {
    let i = i as u32;
    capnp_body_expr(clause_args.borrow().get(i), arg);
  }
}
