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
  Uint64V(u64),
  HStringV(String),
  Blob(&'a [u8])
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
