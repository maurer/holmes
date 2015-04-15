extern crate capnpc;
use std::path::Path;

fn main() {
  ::capnpc::compile(Path::new("src"),
                    &[Path::new("src/holmes.capnp")]).unwrap();
}
