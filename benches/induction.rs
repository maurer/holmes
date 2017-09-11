#[macro_use]
extern crate holmes;
use holmes::simple::*;
use std::time::Instant;

fn run_induction(size: u64) {
    single(&|holmes: &mut Engine, core: &mut Core| {
        holmes_exec!(holmes, {
            predicate!(p(uint64));
            predicate!(q(uint64))
        })?;
        for i in 0..(size - 1) {
            rule!(holmes, p((i + 1)) <= p((i)) & p((i + 1)))?;
            rule!(holmes, q((i + 1)) <= q((i)) & p((i)))?;
        }
        holmes.new_fact(&Fact {
            pred_name: "p".to_string(),
            args: vec![0.to_value()],
        })?;
        holmes.new_fact(&Fact {
            pred_name: "q".to_string(),
            args: vec![1.to_value()],
        })?;

        core.run(holmes.quiesce()).unwrap();

        Ok(())
    })
}

fn induction(size: u64) {
    let now = Instant::now();
    run_induction(size);
    let one = now.elapsed();
    println!("induction({}): {:?}", size, one)
}

fn main() {
    println!("Warning: Results not statistically valid");
    for i in &[10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
        induction(*i)
    }
}
