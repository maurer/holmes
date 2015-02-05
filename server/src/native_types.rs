use capnp::list::{struct_list};
use holmes_capnp::holmes;

use std::str::FromStr;
use std::string::{ToString, String};

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

pub fn convert_vals<'a> (args_reader : struct_list::Reader<'a, holmes::val::Reader<'a>>)
  -> Vec<HValue<'a>> {
  let mut args = Vec::new();
  for arg_reader in args_reader.iter() {
    match arg_reader.which() {
      Some(holmes::val::Uint64(v)) => args.push(UInt64V(v)),
      Some(holmes::val::String(s)) => args.push(HStringV(s)),
      Some(holmes::val::Blob(b))   => args.push(BlobV(b)),
      None => () //TODO what should we do if there's an unknown value?
    }
  }
  args
}
