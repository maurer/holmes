#[macro_use]
extern crate holmes;
use holmes::simple::*;

// Bug #10
// Originally generated:
// SELECT t0.id, t1.id, t2.id, t0.arg0, t0.arg2, t1.arg1, t1.arg3, 0 FROM facts.assoc as t0 JOIN
// facts.out as t2 ON t2.arg0 = t0.arg0 AND t2.arg1 = t1.arg1 AND t2.arg2 = t0.arg2 JOIN facts.look
// as t1 ON t1.arg0 = t0.arg0 WHERE not exists (select 1 from cache.rule1 WHERE id0 = t0.id AND id1
// = t1.id AND id2 = t2.id)
//
// Bug fixed by moving all ON clauses to the last one.
#[test]
fn misordered_join() {
    single(&|holmes: &mut Engine, core: &mut Core| {
        holmes_exec!(holmes, {
    predicate!(out(string, uint64, uint64));
    predicate!(assoc(string, uint64, uint64));
    predicate!(look(string, uint64, uint64, uint64));
    rule!(out_step: out(name, addr, next) <=
             assoc(name, [_], tgt) &
             look(name, addr, [_], next) &
             out(name, addr, tgt))
    })?;
        core.run(holmes.quiesce()).unwrap();
        Ok(())
    })
}
