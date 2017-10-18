//! Holmes/Datalog Execution Engine
//!
//! This module contains the logic for rule execution and non-persistent state
//! maintenance.

pub mod types;

use std::collections::hash_map::HashMap;
use pg::dyn::{Type, Value};
use pg::dyn::values;
use self::types::{BindExpr, Clause, Expr, Fact, Func, MatchExpr, Predicate, Rule};
use pg::{FactId, PgDB};
use tokio_core::reactor::Handle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use futures::{Async, Future, Poll, Stream};
use futures::future::{FutureResult, result};
use futures::task::{Task, current};
use std::time::{Instant, Duration};

#[derive(Clone, Copy, PartialEq, Debug)]
enum RuleState {
    Idle,
    Running,
    Queued,
    ShutDown,
}

#[derive(Clone, Debug)]
struct Signal {
    state: Rc<Cell<RuleState>>,
    referents: Rc<RefCell<Vec<Task>>>,
    task: Rc<RefCell<Option<Task>>>,
}

impl Signal {
    fn new() -> Self {
        Signal {
            state: Rc::new(Cell::new(RuleState::Idle)),
            referents: Rc::new(RefCell::new(Vec::new())),
            task: Rc::new(RefCell::new(None)),
        }
    }

    fn refer(&self, task: Task) {
        self.referents.borrow_mut().push(task)
    }

    fn await(&self, task: Task) {
        // Only one task can await a signal, if there's already
        // one waiting, there's been a programming error
        assert!(self.task.borrow().is_none());
        *self.task.borrow_mut() = Some(task)
    }

    fn signal(&self) {
        if self.state.get() != RuleState::ShutDown {
            trace!("Queuing new work");
            self.state.set(RuleState::Queued);
            // If the target of this signal is blocked, unblock it
            match self.task.borrow_mut().take() {
                Some(t) => t.notify(),
                None => (),
            }
        }
    }

    fn go_dormant(&self) {
        // We went idle, let anyone waiting for this know
        for task in self.referents.borrow().iter() {
            task.notify();
        }

        // They'll wake up from the notify, and so can let us
        // know if they need to be woken up again.
        self.referents.borrow_mut().truncate(0);
    }

    fn done(&self) -> FutureResult<(), ()> {
        trace!("Done with work loop");
        if self.state.get() == RuleState::Running {
            trace!("And no new work arrived, going idle");
            self.state.set(RuleState::Idle);
            self.go_dormant();
        }
        result(Ok(()))
    }

    fn stop(&self) -> FutureResult<(), ()> {
        trace!("Work loop being terminated");
        self.state.set(RuleState::ShutDown);
        self.go_dormant();
        result(Ok(()))
    }

    fn dormant(&self) -> bool {
        (self.state.get() == RuleState::Idle) || (self.state.get() == RuleState::ShutDown)
    }
}

impl Stream for Signal {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        trace!("Asking about new work");
        use self::RuleState::*;
        match self.state.get() {
            Idle => {
                trace!("None yet");
                self.await(current());
                Ok(Async::NotReady)
            }
            Running => panic!("Tried to ask for more work while still running"),
            ShutDown => Ok(Async::Ready(None)),
            Queued => {
                trace!("New work arrived, waking up");
                self.state.set(Running);
                Ok(Async::Ready(Some(())))
            }
        }
    }
}

/// Future representing the quiescence of the Holmes engine
/// See `Engine::quiesce()` to create one
pub struct Quiescence {
    signals: Vec<Signal>,
}

impl Quiescence {
    fn new(signals: Vec<Signal>) -> Self {
        Quiescence { signals: signals }
    }
}

impl Future for Quiescence {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        trace!("Checking quiescence");
        for signal in self.signals.iter() {
            if !signal.dormant() {
                signal.refer(current());
                return Ok(Async::NotReady);
            }
        }
        Ok(Async::Ready(()))
    }
}

#[derive(Debug, Clone)]
/// RuleProfile contains execution information about a single rule
pub struct RuleProfile {
    /// Name of the rule under profile
    pub name: String,
    /// Total time spent performing SQL fetch operations
    pub select_time: Duration,
    /// Total time spent pushing results to the database
    pub insert_time: Duration,
    /// Total amount of time spent inside the rule's code.
    /// If this is not close to compute_time+sql_time,
    /// something is missing from the profile.
    pub rule_time: Duration,
    /// Total amount of time spent evaluating where clauses
    pub compute_time: Duration,
    /// Worst case SQL fetch
    pub max_select_time: Duration,
    /// Worst case SQL insert
    pub max_insert_time: Duration,
    /// Worst case where clause
    pub max_compute_time: Duration,
}

impl RuleProfile {
    pub fn new(name: String) -> Self {
        RuleProfile {
            name: name,
            select_time: Duration::new(0, 0),
            insert_time: Duration::new(0, 0),
            rule_time: Duration::new(0, 0),
            compute_time: Duration::new(0, 0),
            max_select_time: Duration::new(0, 0),
            max_insert_time: Duration::new(0, 0),
            max_compute_time: Duration::new(0, 0),
        }
    }
    pub fn add_insert_time(&mut self, d: Duration) {
        self.insert_time += d;
        if d > self.max_insert_time {
            self.max_insert_time = d;
        }
    }
    pub fn add_select_time(&mut self, d: Duration) {
        self.select_time += d;
        if d > self.max_select_time {
            self.max_select_time = d;
        }
    }
    pub fn add_compute_time(&mut self, d: Duration) {
        self.compute_time += d;
        if d > self.max_compute_time {
            self.max_compute_time = d;
        }
    }
    pub fn add_rule_time(&mut self, d: Duration) {
        self.rule_time += d;
    }
}

/// The `Engine` type contains the context necessary to run a Holmes program
pub struct Engine {
    fact_db: Rc<PgDB>,
    funcs: HashMap<String, Rc<Func>>,
    rules: HashMap<String, Rc<RefCell<Vec<Signal>>>>,
    rule_profiles: Vec<Rc<RefCell<RuleProfile>>>,
    signals: Vec<Signal>,
    event_loop: Handle,
    start_time: Instant,
    limiter: Option<Duration>,
}

#[allow(missing_docs)]
mod errors {
    use pg;
    use postgres;
    error_chain! {
        errors {
            Invalid(msg: String) {
                description("Invalid Request")
                display("Invalid Request: {}", msg)
            }
            Internal(msg: String) {
                description("Internal Error (bug)")
                display("Internal Error (bug): {}", msg)
            }
            Type(msg: String) {
                description("Type Error")
                display("Type Error: {}", msg)
            }
        }
        foreign_links {
            FactDB(pg::Error);
            Postgres(postgres::error::Error);
        }
    }
}

pub use self::errors::*;

fn substitute(clause: &Clause, ans: &Vec<Value>) -> Fact {
    use self::types::MatchExpr::*;
    Fact {
        pred_name: clause.pred_name.clone(),
        args: clause
            .args
            .iter()
            .map(|m_expr| match *m_expr {
                Unbound => panic!("Unbound is not allowed in substituted facts"),
                Var(ref n) => ans[*n as usize].clone(),
                Const(ref v) => v.clone(),
            })
            .collect(),
    }
}

impl Engine {
    /// Create a fresh engine by handing it a fact database to use
    pub fn new(db: PgDB, handle: Handle) -> Self {
        Engine {
            fact_db: Rc::new(db),
            funcs: HashMap::new(),
            rules: HashMap::new(),
            signals: Vec::new(),
            rule_profiles: Vec::new(),
            event_loop: handle,
            start_time: Instant::now(),
            limiter: None,
        }
    }

    pub fn run_sql(&self, path: &str) {
        use std::io::Read;
        let mut fd = ::std::fs::File::open(path).unwrap();
        let mut sql = String::new();
        fd.read_to_string(&mut sql);
        let conn = self.fact_db.conn().unwrap();
        conn.batch_execute(&sql).unwrap();
    }

    /// For correct operation, limit_time must be called before the installation of
    /// any rules
    pub fn limit_time(&mut self, limiter: Duration) {
        self.limiter = Some(limiter)
    }

    /// Dump profiling information for how much time was spent in each rule
    pub fn dump_profile(&self) -> Vec<RuleProfile> {
            self.rule_profiles.iter().map(|x| x.borrow().clone()).collect()
    }

    /// Seach the type registry for a named type
    /// If present, it returns `Some(type)`, otherwise `None`
    pub fn get_type(&self, name: &str) -> Option<Type> {
        self.fact_db.get_type(name)
    }
    /// Register a new type
    /// This type must be a named type (e.g. type.name() should return `Some`)
    pub fn add_type(&self, type_: Type) -> Result<()> {
        Ok(self.fact_db.add_type(type_)?)
    }
    /// Register a new predicate
    /// This defines the type signature of a predicate and persists it
    ///
    /// * Predicates must have at least one argument
    /// * Predicates must have a unique name
    /// * While using the `pg` backend, their name must be lowercase ascii or '_'
    pub fn new_predicate(&self, pred: &Predicate) -> Result<()> {

        // Verify we have at least one argument
        if pred.fields.len() == 0 {
            bail!(ErrorKind::Invalid(
                "Predicates must have at least one argument.".to_string(),
            ));
        }

        // Check for existing predicates/type issues
        match self.fact_db.get_predicate(&pred.name) {
            Some(p) => {
                if pred.fields == p.fields {
                    // TODO should this be return ()
                    ()
                } else {
                    bail!(ErrorKind::Type(
                        format!("{:?} != {:?}", pred.fields, p.fields),
                    ));
                }
            }
            None => (),
        }

        Ok(self.fact_db.new_predicate(pred)?)
    }

    /// Retrieves a named predicate from the database. This is primarily of use for
    /// retrieving metadata about a predicate for display.
    pub fn get_predicate(&self, name: &str) -> Result<Option<Predicate>> {
        Ok(self.fact_db.get_predicate(name))
    }

    fn get_dep_rules(&mut self, pred: &String) -> Rc<RefCell<Vec<Signal>>> {
        self.rules
            .entry(pred.to_string())
            .or_insert(Rc::new(RefCell::new(Vec::new())))
            .clone()
    }

    /// Adds a new fact to the database
    /// If the fact is already present, a new copy will not be added.
    ///
    /// * The relevant predicate must already be registered
    /// * The fact must be correctly typed
    pub fn new_fact(&mut self, fact: &Fact) -> Result<()> {
        match self.fact_db.get_predicate(&fact.pred_name) {
            Some(ref pred) => {
                if (fact.args.len() != pred.fields.len()) ||
                    (!fact.args.iter().zip(pred.fields.iter()).all(
                        |(val, field)| {
                            val.type_() == field.type_.clone()
                        },
                    ))
                {
                    bail!(ErrorKind::Type(format!(
                        "Fact ({:?}) does not \
                                                   match predicate ({:?})",
                        fact,
                        pred.fields
                    )));
                }
            }
            None => bail!(ErrorKind::Invalid("Predicate not registered".to_string())),
        }
        {
            if self.fact_db.insert_fact(&fact)?.is_some() {
                let deps = self.get_dep_rules(&fact.pred_name);
                for signal in deps.borrow().iter() {
                    signal.signal();
                }
            }
            Ok(())
        }
    }

    /// Returns success in the appropriate type. This helper function is to
    /// support the EDSL, and it is not anticipated to be useful normally.
    pub fn nop(&self) -> Result<()> {
        Ok(())
    }

    /// Given a query (similar to the rhs of a rule in Datalog), provide the set
    /// of satisfying answers in the database.
    pub fn derive(&self, query: &Vec<Clause>) -> Result<Vec<Vec<Value>>> {
        let outs = self.fact_db.search_facts(query, None)?;
        let res = outs.into_iter().map(|x| x.1).collect();
        Ok(res)
    }

    /// Render a predicate as an html table
    pub fn render(&self, pred_name: &String) -> Result<String> {
        let pred = self.get_predicate(pred_name)?.ok_or(ErrorKind::Invalid(
            "Predicate absent".to_string(),
        ))?;
        let data = self.derive(&vec![
            Clause {
                pred_name: pred_name.to_string(),
                args: pred.fields
                    .iter()
                    .enumerate()
                    .map(|(i, _)| MatchExpr::Var(i))
                    .collect(),
            },
        ])?;
        let descr = match pred.description {
            Some(descr) => format!("<h3>{}</h3><br />", descr),
            None => "".to_string(),
        };
        let mut html = format!("<h1>{}:</h1><br />{}<table><tr>", pred_name, descr);
        for field in pred.fields {
            let name = match field.name {
                Some(ref name) => name,
                None => "N/A",
            };
            let descr = match field.description {
                Some(ref descr) => format!(" title=\"{}\"", descr),
                None => "".to_string(),
            };
            html.push_str(&format!("<th{}>{}</th>", descr, name));
        }
        html.push_str("</tr>");
        for row in data {
            html.push_str("<tr>");
            for col in row {
                html.push_str(&format!("<td>{}</td>", col))
            }
        }
        html.push_str("</table>");
        Ok(html)
    }

    /// Register a new rule with the database
    pub fn new_rule(&mut self, rule: &Rule) -> Result<()> {
        trace!("Registering rule: {:?}", rule);
        let signal = Signal::new();
        let trigger = signal.clone();
        let profile = Rc::new(RefCell::new(RuleProfile::new(rule.name.clone())));
        self.rule_profiles.push(profile.clone());
        self.signals.push(signal.clone());

        for pred in &rule.body {
            let dep_rules = self.get_dep_rules(&pred.pred_name);
            dep_rules.borrow_mut().push(signal.clone());
        }

        let rule_future = {
            let mut next_fact_id = None;
            let fdb = self.fact_db.clone();
            let funcs = self.funcs.clone();
            let buddies = self.get_dep_rules(&rule.head.pred_name);
            let rule = rule.clone();
            let out_signal = signal.clone();
            let start_time = self.start_time.clone();
            let limiter = self.limiter.clone();
            signal.for_each(move |_| {
                let rule_start = Instant::now();
                match (start_time.elapsed(), limiter) {
                    (run_time, Some(limit_time)) if run_time > limit_time => return out_signal.stop(),
                    _ => ()
                }
                trace!("Activating rule: {:?}", rule.name);
                let mut productive: usize = 0;
                    let pre_db = Instant::now();
                    let states_0 = fdb.search_facts(&rule.body, next_fact_id).chain_err(|| format!("Search from {}", rule.name)).unwrap();
                    let sql_time = pre_db.elapsed();
                    profile.borrow_mut().add_select_time(sql_time);
                    next_fact_id = states_0.iter().flat_map(|x| x.0.iter()).max().map(|x| x + 1).or(next_fact_id);
                let results = states_0.len();
                    trace!("Query submitted");
                    let mut states: Box<
                        Iterator<
                            Item = (Vec<FactId>,
                                    Vec<Value>),
                        >,
                    > = Box::new(states_0.into_iter());
                    let where_start = Instant::now();
                    for where_clause in rule.wheres.iter() {
                        let wc = where_clause.clone();
                        let bf = &funcs;
                        let next_states = states.flat_map(move |state| {
                            let resp = eval(&wc.rhs, &state.1, bf);
                            let out: Vec<_> = bind(&wc.lhs, resp, &state.1)
                                .into_iter()
                                .map(|x| (state.0.clone(), x))
                                .collect();
                            out
                        });
                        states = Box::new(next_states);
                    }
                    let facts: Vec<Fact> = states.map(|state| substitute(&rule.head, &state.1)).collect();
                    let compute_time = where_start.elapsed();
                    profile.borrow_mut().add_compute_time(compute_time);
                    trace!("Insertions beginning");
                    let insert_start = Instant::now();
                    for fact in facts {
                        if fdb.insert_fact(&fact)
                            .unwrap()
                            .is_some()
                        {
                            productive += 1;
                        }
                    }
                    let insert_time = insert_start.elapsed();
                    profile.borrow_mut().add_insert_time(insert_time);
                    trace!("Insertions done");
                trace!(
                    "Generated {} results, turned into {} facts.",
                    results,
                    productive
                );

                if productive > 0 {
                    for buddy in buddies.borrow().iter() {
                        buddy.signal();
                    }
                }

                let rule_elapsed = rule_start.elapsed();
                profile.borrow_mut().add_rule_time(rule_elapsed);
                out_signal.done()
            })
        };

        self.event_loop.spawn(rule_future);
        trigger.signal();
        Ok(())
    }

    /// Register a new function with the database, to be called from within a
    /// rule
    ///
    /// Do not attempt to register a function name multiple times.
    // TODO: stop function reregistration, document restriction
    pub fn reg_func(&mut self, name: String, func: Func) -> Result<()> {
        self.funcs.insert(name, Rc::new(func));
        Ok(())
    }

    /// Creates a quiescence future to be run on the event loop provided when
    /// the engine was created. The future will only gaurantee quiescence upon
    /// completion so long as no new rules have been added.
    pub fn quiesce(&self) -> Quiescence {
        Quiescence::new(self.signals.clone())
    }
}

// In an assignment statement, once the rhs has been computed, binds the
// rhs value onto the expression on the left, using the state to check that
// already bound variables are bound to the same things
// It returns list of output states, each of which is a list of var bindings
fn bind(lhs: &BindExpr, rhs: Value, state: &Vec<Value>) -> Vec<Vec<Value>> {
    use self::types::BindExpr::*;
    use self::types::MatchExpr::*;
    match *lhs {
        // If we are unbound, we no-op
        Normal(Unbound) => vec![state.clone()],
        // To bind to a variable,
        Normal(Var(v)) => {
            // If the variable is defined, check equality
            if v < state.len() {
                if state[v] == rhs {
                    vec![state.clone()]
                } else {
                    vec![]
                }
            // If the variable is to be defined, define it
            } else if v == state.len() {
                let mut next = state.clone();
                next.push(rhs.clone());
                vec![next]
            // Otherwise it is a malformed binding
            } else {
                panic!("Variable out of range")
            }
        }
        Normal(Const(ref v)) => {
            if *v == rhs {
                vec![state.clone()]
            } else {
                vec![]
            }
        }
        Destructure(ref lhss) => {
            let rhss = match rhs.get().downcast_ref::<Vec<Value>>() {
                Some(ref rhss) => rhss.iter(),
                _ => panic!("Attempted to destructure non-list"),
            };
            let mut next = vec![state.clone()];
            for (lhs, rhs) in lhss.iter().zip(rhss) {
                let mut next_next = vec![];
                for state in next {
                    next_next.extend(bind(lhs, rhs.clone(), &state));
                }
                next = next_next;
            }
            next
        }
        Iterate(ref inner) => {
            let rhss = match rhs.get().downcast_ref::<Vec<Value>>() {
                Some(ref rhss) => rhss.iter(),
                _ => panic!("Attempted to destructure non-list"),
            };
            rhss.flat_map(|rhs| bind(inner, rhs.clone(), &state))
                .collect()
        }
    }
}

// Evaluates an expression, given a set of bindings to variables
fn eval(expr: &Expr, subs: &Vec<Value>, funcs: &HashMap<String, Rc<Func>>) -> Value {
    use self::types::Expr::*;
    match *expr {
        Var(var) => subs[var as usize].clone(),
        Val(ref val) => val.clone(),
        App(ref fun_name, ref args) => {
            let arg_vals: Vec<Value> = args.iter()
                .map(|arg_expr| eval(arg_expr, subs, funcs))
                .collect();
            let arg = if arg_vals.len() == 1 {
                arg_vals[0].clone()
            } else {
                values::Tuple::new(arg_vals) as Value
            };
            (funcs[fun_name].run)(arg)
        }
    }
}
