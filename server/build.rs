#![allow(unstable)]

extern crate capnpc;

fn main() {
  ::capnpc::compile(Path::new(""),
                    &[Path::new("holmes.capnp")]).unwrap();
}
