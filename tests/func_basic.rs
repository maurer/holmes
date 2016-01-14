use common::*;

#[test]
pub fn reg_func() {
  single(&|holmes : &mut Holmes| {
    func!(holmes,
      let test_func : uint64 -> uint64 =
        |_v : HValue| {
          42.to_hvalue()
        })
  })
}
