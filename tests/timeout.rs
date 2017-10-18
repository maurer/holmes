#[macro_use]
extern crate holmes;
use holmes::simple::*;
use std::time::{Duration, Instant};

// The whole point of this test is to take an excessively long execution
// and make sure it terminates about on time if a limiter is provided
#[test]
pub fn infinity() {
    single(&|holmes: &mut Engine, core: &mut Core| {
        // Amount of time holmes gets
        let limit = Duration::new(2, 0);
        // Wiggle room to shut things down
        let wiggle = Duration::new(1, 0);
        holmes.limit_time(limit.clone());
        let start = Instant::now();
        try!(holmes_exec!(holmes, {
      predicate!(count(uint64));
      fact!(count(0));
      func!(let inc: uint64 -> uint64 = |i: &u64| *i + 1);
      rule!(inc: count(n_plus_one) <= count(n), {
          let n_plus_one = {inc([n])}
      })
    }));
        core.run(holmes.quiesce()).unwrap();
        assert!(start.elapsed() < limit + wiggle);
        Ok(())
    })
}
