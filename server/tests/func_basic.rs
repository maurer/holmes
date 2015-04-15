use common::*;

use holmes::client::*;
use holmes::native_types::*;
use holmes::native_types::HType::*;
use holmes::native_types::HValue::*;

#[test]
pub fn reg_func() {
  server_single(&|client : &mut Client| {
    &client.new_func("test_func", HFunc {
      input_types : vec![UInt64],
      output_types : vec![UInt64],
      run : Box::new(|_v : Vec<HValue>| {
        vec![UInt64V(42)]
      })
    }).unwrap();
  })
}
