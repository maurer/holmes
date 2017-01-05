use common::*;

#[test]
pub fn reg_func() {
    single(&|holmes: &mut Engine| {
        func!(holmes,
      let test_func : uint64 -> uint64 =
        |_v : &u64| {
          42 as u64
        })
    })
}
