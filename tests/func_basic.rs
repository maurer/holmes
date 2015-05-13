use common::*;

use holmes::client::*;
use holmes::native_types::*;

#[test]
pub fn reg_func() {
  server_single(&|client : &mut Client| {
    func!(client,
      let test_func : [uint64] -> [uint64] =
        |_v : Vec<HValue>| {
          vec![42.to_hvalue()]
        }).unwrap();
  })
}
