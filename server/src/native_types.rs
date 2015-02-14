use capnp::struct_list;
use holmes_capnp::holmes;

use std::str::FromStr;
use std::string::{ToString, String};

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
      _ => Err(s.to_string())
    }
  }
}

impl ToString for HType {
  fn to_string(&self) -> String {
    String::from_str(match self {
      &UInt64  => {"uint64"}
      &HString => {"string"}
      &Blob    => {"blob"}
    })
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
pub struct Rule {
  pub head : Clause,
  pub body : Vec<Clause>
}

pub fn convert_types<'a> (types_reader : struct_list::Reader<'a, holmes::h_type::Reader<'a>>)
   -> Vec<HType> {
  let mut types = Vec::new();
  for type_reader in types_reader.iter() {
    match type_reader.which() {
      Some(holmes::h_type::Uint64(())) => {types.push(UInt64);}
      Some(holmes::h_type::String(())) => {types.push(HString);}
      Some(holmes::h_type::Blob(())) => {types.push(Blob);}
      None => { } //TODO: What should we do if there's an unknown type?
    }
  }
  types
}

pub fn convert_val<'a> (val_reader : holmes::val::Reader<'a>)
  -> HValue {
  match val_reader.which() {
    Some(holmes::val::Uint64(v)) => UInt64V(v),
    Some(holmes::val::String(s)) => HStringV(s.to_string()),
    Some(holmes::val::Blob(b)) => {
      let mut bv = Vec::new();
      bv.push_all(b);
      BlobV(bv)
    }
    None => panic!("Invalid value on wire")
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

pub fn convert_vals<'a> (args_reader : struct_list::Reader<'a, holmes::val::Reader<'a>>)
  -> Vec<HValue> {
  let mut args = Vec::new();
  for arg_reader in args_reader.iter() {
    args.push(convert_val(arg_reader));
  }
  args
}

pub fn convert_clause<'a>(clause_reader : holmes::body_clause::Reader<'a>)
                         -> Clause {
  let pred = clause_reader.get_predicate();
  let exprs_reader = clause_reader.get_args();
  let mut args = Vec::new();
  for expr_reader in exprs_reader.iter() {
    let match_expr = match expr_reader.which() {
      Some(holmes::body_expr::Unbound(())) => Unbound,
      Some(holmes::body_expr::Var(v)) => Var(v),
      Some(holmes::body_expr::Const(val)) =>
        HConst(convert_val(val)),
      None => panic!("Unknown expr type")
    };
    args.push(match_expr);
  }
  Clause {
    pred_name : pred.to_string(),
    args : args
  }
}
pub fn convert_clauses<'a>(clauses_reader : struct_list::Reader<'a,
                       holmes::body_clause::Reader<'a>>) ->
                       Vec<Clause> {
  clauses_reader.iter().map(convert_clause).collect()
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
    head : convert_clause(rule_reader.get_head()),
    body : convert_clauses(rule_reader.get_body())
  }
}
pub fn capnp_expr<'a>(mut expr_builder : holmes::body_expr::Builder<'a>,
                  expr : &MatchExpr) {
  match expr {
    &Unbound => expr_builder.set_unbound(()),
    &Var(v) => expr_builder.set_var(v),
    &HConst(ref val) => capnp_val(expr_builder.init_const(), val)
  }
}

pub fn capnp_clause<'a>(mut clause_builder : holmes::body_clause::Builder<'a>,
                        clause : &Clause) {
  clause_builder.set_predicate(&clause.pred_name[]);
  let mut clause_args = clause_builder.init_args(clause.args.len() as u32);
  for (i, arg) in clause.args.iter().enumerate() {
    let i = i as u32;
    capnp_expr(clause_args.borrow().get(i), arg);
  }
}
