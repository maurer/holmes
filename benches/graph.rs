#[macro_use]
extern crate holmes;
use holmes::simple::*;
use std::time::Instant;
use std::thread;

fn run_graph(size: u64) {
    single(&|holmes: &mut Engine| {
        predicate!(holmes, reachable(uint64, uint64))?;
        predicate!(holmes, edge(uint64, uint64))?;
        predicate!(holmes, increasing(uint64, uint64))?;

        for i in 0..(size - 1) {
            holmes.new_fact(&Fact {
                    pred_name: "edge".to_string(),
                    args: vec![i.to_value(), (i + 1).to_value()],
                })?;
        }

        holmes_exec!(holmes, {
            rule!(reachable(X, Y) <= edge(X, Y));
            rule!(reachable(X, Y) <= edge(X, Z) & reachable(Z, Y));
            func!(let lt : (uint64, uint64) -> bool = |(x, y): (&u64, &u64)| {
                x < y
            });
            rule!(increasing(X, Y) <= edge(X, Y), {
                let (true) = {lt([X], [Y])}
            });
            rule!(increasing(X, Y) <= edge(X, Z) & increasing(Z, Y), {
                let (true) = {lt([X], [Z])}
            })
        })
    })
}

fn graph(size: u64) {
    let now = Instant::now();
    run_graph(size);
    let one = now.elapsed();
    println!("graph({}): {:?}", size, one)
}

fn main() {
    thread::Builder::new()
        .stack_size(1024 * 1024 * 1024)
        .spawn(|| {
            println!("Warning: Results not statistically valid");
            for i in &[10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
                graph(*i)
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
