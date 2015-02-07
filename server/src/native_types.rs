use capnp::list::{struct_list};
use holmes_capnp::holmes;

use std::str::FromStr;
use std::string::{ToString, String};

use std::borrow::{ToOwned, BorrowFrom};

pub type PredId = u64;

#[derive(Copy,PartialEq,Clone)]
pub enum HType {
  UInt64,
  HString,
  Blob
}
use native_types::HType::*;

#[derive(PartialEq,Clone)]
pub enum HValue<'a> {
  UInt64V(u64),
  HStringV(&'a str),
  BlobV(&'a [u8])
}
use native_types::HValue::*;

pub enum OHValue {
  UInt64OV(u64),
  HStringOV(String),
  BlobOV(Vec<u8>)
}
use native_types::OHValue::*;

impl<'a> BorrowFrom<OHValue> for HValue<'a> {
  fn borrow_from(_oh : &OHValue) -> &HValue<'a> {
    unimplemented!()
  }
}

impl<'a> ToOwned<OHValue> for HValue<'a> {
  fn to_owned(&self) -> OHValue {
    match self {
      &HStringV(str) => HStringOV(str.to_string()),
      &UInt64V(i) => UInt64OV(i),
      &BlobV(s) => BlobOV(s.to_owned())
    }
  }
}

pub fn type_check<'a>(vty : (&HValue<'a>, &HType)) -> bool {
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

#[derive(PartialEq,Clone)]
pub struct Predicate {
  pub name  : String,
  pub types : Vec<HType>
}

#[derive(PartialEq,Clone)]
pub struct Fact<'a> {
  pub pred_name : String,
  pub args : Vec<HValue<'a>>
}

pub type HVar = u32;

pub enum MatchExpr<'a> {
  Unbound,
  Var(HVar),
  HConst(HValue<'a>)
}
use native_types::MatchExpr::*;

pub struct Clause<'a> {
  pub pred_name : String,
  pub args : Vec<MatchExpr<'a>>
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
  -> HValue<'a> {
  match val_reader.which() {
    Some(holmes::val::Uint64(v)) => UInt64V(v),
    Some(holmes::val::String(s)) => HStringV(s),
    Some(holmes::val::Blob(b)) => BlobV(b),
    None => panic!("Invalid value on wire")
  }
}

pub fn capnp_val<'a> (mut val_builder : holmes::val::Builder<'a>,
                      h_val : &HValue<'a>) {
  match h_val {
    &HStringV(x) => val_builder.set_string(x),
    &BlobV(x) => val_builder.set_blob(x),
    &UInt64V(x) => val_builder.set_uint64(x)
  }
}

pub fn convert_vals<'a> (args_reader : struct_list::Reader<'a, holmes::val::Reader<'a>>)
  -> Vec<HValue<'a>> {
  let mut args = Vec::new();
  for arg_reader in args_reader.iter() {
    args.push(convert_val(arg_reader));
  }
  args
}

pub fn convert_clauses<'a>(clauses_reader : struct_list::Reader<'a,
                       holmes::body_clause::Reader<'a>>) ->
                       Vec<Clause<'a>> {
  let mut clauses = Vec::new();
  for clause_reader in clauses_reader.iter() {
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
    clauses.push(Clause {
      pred_name : pred.to_string(),
      args : args
    });
  }
  clauses
}

pub fn capnp_expr<'a>(mut expr_builder : holmes::body_expr::Builder<'a>,
                  expr : &MatchExpr<'a>) {
  match expr {
    &Unbound => expr_builder.set_unbound(()),
    &Var(v) => expr_builder.set_var(v),
    &HConst(ref val) => capnp_val(expr_builder.init_const(), val)
  }
}

pub fn capnp_clause<'a, 'b>(mut clause_builder : holmes::body_clause::Builder<'a>,
                        clause : &Clause<'b>) {
  clause_builder.set_predicate(&clause.pred_name[]);
  let mut clause_args = clause_builder.init_args(clause.args.len() as u32);
  for (i, arg) in clause.args.iter().enumerate() {
    let i = i as u32;
    capnp_expr(clause_args.borrow().get(i), arg);
  }
}
