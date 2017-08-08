#[macro_use]
extern crate holmes;
use holmes::simple::*;
use std::time::Instant;

fn run_clique(size: u64) {
    single(&|holmes: &mut Engine, core: &mut Core| {
        predicate!(holmes, reachable(uint64, uint64))?;
        predicate!(holmes, edge(uint64, uint64))?;
        predicate!(holmes, same_clique(uint64, uint64))?;
        for i in 0..(size - 1) {
            holmes.new_fact(&Fact {
                pred_name: "edge".to_string(),
                args: vec![i.to_value(), (i + 1).to_value()],
            })?;
        }
        fact!(holmes, edge((size - 1), 0))?;
        holmes_exec!(holmes, {
            rule!(reachable(X, Y) <= edge(X, Y));
            rule!(reachable(X, Y) <= edge(X, Z) & reachable(Z, Y));
            rule!(same_clique(X, Y) <= reachable(X, Y) & reachable(Y, X))
        })?;

        core.run(holmes.quiesce()).unwrap();

        Ok(())
    })
}

fn clique(size: u64) {
    let now = Instant::now();
    run_clique(size);
    let one = now.elapsed();
    println!("clique({}): {:?}", size, one)
}

fn main() {
    println!("Warning: Results not statistically valid");
    for i in &[10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
        clique(*i)
    }
}
