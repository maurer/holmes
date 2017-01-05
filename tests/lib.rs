#[macro_use]
extern crate holmes;
extern crate postgres;
extern crate url;

mod common;
mod trivial;
mod predicate_basic;
mod fact_basic;
mod rule_basic;
mod func_basic;
mod rule_where;
mod type_extend;
mod rule_substr;
mod reboot;

mod bugs;
