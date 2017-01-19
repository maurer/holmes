#[macro_use]
extern crate holmes;
use holmes::simple::*;
use std::time::Instant;
use std::thread;

fn run_clique(size: u64) {
    single(&|holmes: &mut Engine| {
        predicate!(holmes, reachable(uint64, uint64))?;
        predicate!(holmes, edge(uint64, uint64))?;
        for i in 0..(size - 1) {
            holmes.new_fact(&Fact {
                    pred_name: "edge".to_string(),
                    args: vec![i.to_value(), (i + 1).to_value()],
                })?;
        }
        holmes_exec!(holmes, {
            rule!(reachable(X, Y) <= edge(X, Y));
            rule!(reachable(X, Y) <= edge(X, Z) & reachable(Z, Y));
            rule!(same_clique(X, Y) <= reachable(X, Y) & reachable(Y, X))
        })
    })
}

fn clique(size: u64) {
    let now = Instant::now();
    run_clique(size);
    let one = now.elapsed();
    println!("clique({}): {:?}", size, one)
}

fn main() {
    thread::Builder::new()
        .stack_size(1024 * 1024 * 1024)
        .spawn(|| {
            println!("Warning: Results not statistically valid");
            for i in &[10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
                clique(*i)
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
