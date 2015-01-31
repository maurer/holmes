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

impl FromStr for HType {
  fn from_str(s : &str) -> Option<HType> {
    match s {
      "uint64" => {Some(UInt64)}
      "string" => {Some(HString)}
      "blob"   => {Some(Blob)}
      _ => {None}
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
