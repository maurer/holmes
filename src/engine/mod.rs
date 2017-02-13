//! Holmes/Datalog Execution Engine
//!
//! This module contains the logic for rule execution and non-persistent state
//! maintenance.

pub mod types;

use std::collections::hash_map::HashMap;
use pg::dyn::{Value, Type};
use pg::dyn::values;
use self::types::{Fact, Rule, Func, Predicate, Clause, Expr, BindExpr, Projection, MatchExpr};
use fact_db::{FactDB, CacheId};
use tokio_core::reactor::Handle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use futures::{Stream, Future, Async, Poll, done, BoxFuture};
use futures::task::{park, Task};

#[derive(Clone,Copy,PartialEq,Debug)]
enum RuleState {
    Idle,
    Running,
    Queued,
    ShutDown,
}

#[derive(Clone,Debug)]
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
                Some(t) => t.unpark(),
                None => (),
            }
        }
    }

    fn done(&self) -> BoxFuture<(), ()> {
        trace!("Done with work loop");
        if self.state.get() == RuleState::Running {
            trace!("And no new work arrived, going idle");
            self.state.set(RuleState::Idle);

            // We went idle, let anyone waiting for this know
            for task in self.referents.borrow().iter() {
                task.unpark();
            }

            // They'll wake up from the unpark, and so can let us
            // know if they need to be woken up again.
            self.referents.borrow_mut().truncate(0);
        }
        done(Ok(())).boxed()
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
                self.await(park());
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
                signal.refer(park());
                return Ok(Async::NotReady);
            }
        }
        Ok(Async::Ready(()))
    }
}

/// The `Engine` type contains the context necessary to run a Holmes program
pub struct Engine<FE: ::std::error::Error + Send + 'static, FDB: FactDB<Error = FE>> {
    fact_db: Rc<FDB>,
    funcs: HashMap<String, Rc<Func>>,
    rules: HashMap<String, Rc<RefCell<Vec<Signal>>>>,
    signals: Vec<Signal>,
    event_loop: Handle,
}

#[allow(missing_docs)]
mod errors {
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
            FactDB {
                description("FactDB propagated error")
            }
        }
    }
}

pub use self::errors::*;

fn substitute(clause: &Clause, ans: &Vec<Value>) -> Fact {
    use self::types::MatchExpr::*;
    Fact {
        pred_name: clause.pred_name.clone(),
        args: clause.args
            .iter()
            .enumerate()
            .map(|(idx, &(ref proj, ref slot))| {
                assert_eq!(proj, &Projection::Slot(idx));
                match *slot {
                    Unbound => panic!("Unbound is not allowed in substituted facts"),
                    Var(ref n) => ans[*n as usize].clone(),
                    Const(ref v) => v.clone(),
                }
            })
            .collect(),
    }
}

impl<FE, FDB> Engine<FE, FDB>
    where FE: ::std::error::Error + Send + 'static,
          FDB: FactDB<Error = FE>
{
    /// Create a fresh engine by handing it a fact database to use
    pub fn new(db: FDB, handle: Handle) -> Self {
        Engine {
            fact_db: Rc::new(db),
            funcs: HashMap::new(),
            rules: HashMap::new(),
            signals: Vec::new(),
            event_loop: handle,
        }
    }

    /// Seach the type registry for a named type
    /// If present, it returns `Some(type)`, otherwise `None`
    pub fn get_type(&self, name: &str) -> Option<Type> {
        self.fact_db.get_type(name)
    }
    /// Register a new type
    /// This type must be a named type (e.g. type.name() should return `Some`)
    pub fn add_type(&self, type_: Type) -> Result<()> {
        Ok(try!(self.fact_db.add_type(type_).chain_err(|| ErrorKind::FactDB)))
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
            bail!(ErrorKind::Invalid("Predicates must have at least one argument.".to_string()));
        }

        // Check for existing predicates/type issues
        match self.fact_db.get_predicate(&pred.name) {
            Some(p) => {
                if pred.fields == p.fields {
                    // TODO should this be return ()
                    ()
                } else {
                    bail!(ErrorKind::Type(format!("{:?} != {:?}", pred.fields, p.fields)));
                }
            }
            None => (),
        }

        Ok(try!(self.fact_db.new_predicate(pred).chain_err(|| ErrorKind::FactDB)))
    }

    /// Retrieves a named predicate from the database. This is primarily of use for
    /// retrieving metadata about a predicate for display.
    pub fn get_predicate(&self, name: &str) -> Result<Option<Predicate>> {
        Ok(self.fact_db.get_predicate(name))
    }

    fn get_dep_rules(&mut self, pred: &String) -> Rc<RefCell<Vec<Signal>>> {
        self.rules.entry(pred.to_string()).or_insert(Rc::new(RefCell::new(Vec::new()))).clone()
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
                   (!fact.args
                    .iter()
                    .zip(pred.fields.iter())
                    .all(|(val, field)| val.type_() == field.type_.clone())) {
                    bail!(ErrorKind::Type(format!("Fact ({:?}) does not \
                                                   match predicate ({:?})",
                                                  fact,
                                                  pred.fields)));
                }
            }
            None => bail!(ErrorKind::Invalid("Predicate not registered".to_string())),
        }
        {
            if self.fact_db.insert_fact(&fact).chain_err(|| ErrorKind::FactDB)? {
                let signals = self.get_dep_rules(&fact.pred_name);
                for signal in signals.borrow().iter() {
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

    fn rule_cache(&mut self, rule: &Rule) -> Result<CacheId> {
        self.fact_db
            .new_rule_cache(rule.body.iter().map(|clause| clause.pred_name.clone()).collect())
            .chain_err(|| ErrorKind::FactDB)
    }

    /// Given a query (similar to the rhs of a rule in Datalog), provide the set
    /// of satisfying answers in the database.
    pub fn derive(&self, query: &Vec<Clause>) -> Result<Vec<Vec<Value>>> {
        Ok(try!(self.fact_db.search_facts(query, None).chain_err(|| ErrorKind::FactDB))
            .into_iter()
            .map(|x| x.1)
            .collect())
    }

    /// Render a predicate as an html table
    pub fn render(&self, pred_name: &String) -> Result<String> {
        let pred = self.get_predicate(pred_name)?
            .ok_or(ErrorKind::Invalid("Predicate absent".to_string()))?;
        let data = self.derive(&vec![Clause {
                              pred_name: pred_name.to_string(),
                              args: pred.fields
                                  .iter()
                                  .enumerate()
                                  .map(|(i, _)| (Projection::Slot(i), MatchExpr::Var(i)))
                                  .collect(),
                          }])?;
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
    pub fn new_rule(&mut self, rule: &Rule) -> Result<()>
        where FDB: 'static
    {
        let signal = Signal::new();
        let trigger = signal.clone();
        self.signals.push(signal.clone());

        for pred in &rule.body {
            let dep_rules = self.get_dep_rules(&pred.pred_name);
            dep_rules.borrow_mut().push(signal.clone());
        }

        let rule_future = {
            let cache = self.rule_cache(&rule)?;
            let fdb = self.fact_db.clone();
            let funcs = self.funcs.clone();
            let buddies = self.get_dep_rules(&rule.head.pred_name);
            let rule = rule.clone();
            let out_signal = signal.clone();
            signal.for_each(move |_| {
                trace!("Activating rule: {:?}", rule);
                let mut states = fdb.search_facts(&rule.body, Some(cache)).unwrap();
                for where_clause in rule.wheres.iter() {
                    let mut next_states = Vec::new();
                    for state in states {
                        let resp = eval(&where_clause.rhs, &state.1, &funcs);
                        next_states.extend(bind(&where_clause.lhs, resp, &state.1)
                            .into_iter()
                            .map(|x| (state.0.clone(), x)));
                    }
                    states = next_states;
                }
                let mut productive = false;
                for state in states {
                    productive |= fdb.insert_fact(&substitute(&rule.head, &state.1))
                        .unwrap();
                }

                if productive {
                    for buddy in buddies.borrow().iter() {
                        buddy.signal();
                    }
                }

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
