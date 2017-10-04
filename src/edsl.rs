//! Holmes EDSL
//!
//! This module provides a set of macros for more easily writing Holmes
//! programs, avoiding manual construction of all tye types required.

/// Converts an EDSL type specification into a Holmes type object
/// Takes the name of a variable containing a holmes object as the first
/// parameter, and a type description as the second.
///
/// [type] -> list of that type
/// (type0, type1, type2) -> tuple of those types
/// type -> look up type by name in the registry
#[macro_export]
macro_rules! htype {
    ($holmes:ident, [$t:tt]) => {
        ::holmes::pg::dyn::types::List::new(htype!($holmes, $t))
    };
    ($holmes:ident, ($($t:tt),*)) => {
        ::holmes::pg::dyn::types::Tuple::new(vec![$(htype!($holmes, $t)),*])
    };
    ($holmes:ident, $i:ident) => {
        $holmes.get_type(stringify!($i))
        .expect(&format!("Type not present in database: {}", stringify!($i)))
    };
}

/// Shorthand notation for performing many actions with the same holmes context
/// Analogous to a weaker version of the `Reader` monad which cannot return
/// values.
///
/// The first parameter is the holmes object to be used, and the second is
/// a list of the actions to be performed, e.g.
///
/// ```c
/// holmes_exec!(holmes, {
///   predicate!(foo(string, uint64));
///   fact!(foo("bar", 3));
/// });
/// ```
///
/// If any of the actions would error, the *enclosing function* will error out.
/// This is due to a limitation in how the `try!` macro works. (It uses return
/// to error out, rather than a bind-like mechanism).
///
/// This only works because the other macros have both an explicit ident form,
/// and one which generates a function taking a `holmes` parameter instead.
#[macro_export]
macro_rules! holmes_exec {
  ($holmes:ident, { $( $action:expr );* }) => {{
        $( try!($action($holmes)); );*
        $holmes.nop()
  }};
}

#[macro_export]
macro_rules! field {
    ($holmes:ident, [$name:ident $t:tt $descr:expr]) => {{::holmes::engine::types::Field {
        name: Some(stringify!($name).to_string()),
        description: Some($descr.to_string()),
        type_: htype!($holmes, $t)
    }}};
    ($holmes:ident, [$name:ident $t:tt]) => {{::holmes::engine::types::Field {
        name: Some(stringify!($name).to_string()),
        description: None,
        type_: htype!($holmes, $t)
    }}};
    ($holmes:ident, $t:tt) => {{::holmes::engine::types::Field {
        name: None,
        description: None,
        type_: htype!($holmes, $t)
    }}};
}

/// Registers a predicate with the `Holmes` context.
///
/// ```c
/// predicate!(holmes, foo(string, uint64))
/// ```
///
/// will register a predicate named foo, with a `string` slot and a `uint64`
/// slot, to the provided `holmes` context object.
///
/// If the `holmes` parameter is omitted, it will generate a function taking
/// a `holmes` parameter in its stead.
#[macro_export]
macro_rules! predicate {
  ($holmes:ident, $pred_name:ident($($t:tt),*), $descr:expr) => {{
    let fields = vec![$(field!($holmes, $t),)*];
    $holmes.new_predicate(&::holmes::engine::types::Predicate {
      name: stringify!($pred_name).to_string(),
      description: Some($descr.to_string()),
      fields: fields
    })
  }};
  ($holmes:ident, $pred_name:ident($($t:tt),*)) => {{
    let fields = vec![$(field!($holmes, $t),)*];
    $holmes.new_predicate(&::holmes::engine::types::Predicate {
      name: stringify!($pred_name).to_string(),
      description: None,
      fields: fields
    })
  }};
  ($pred_name:ident($($t:tt),*) : $descr:expr) => { |holmes: &mut ::holmes::Engine| {
    predicate!(holmes, $pred_name($($t),*), $descr)
  }};
  ($pred_name:ident($($t:tt),*)) => { |holmes: &mut ::holmes::Engine| {
    predicate!(holmes, $pred_name($($t),*))
  }};
}

/// Stores a fact with the `Holmes` context.
///
/// ```c
/// fact!(holmes, foo("bar", 3))
/// ```
///
/// will store a true instance of the predicate foo with "bar" in the first
/// slot and 3 in the second.
///
/// If the `holmes` parameter is omitted, it will generate a function taking
/// a `holmes` parameter in its stead.
#[macro_export]
macro_rules! fact {
  ($holmes:ident, $pred_name:ident($($a:expr),*)) => {
    $holmes.new_fact(&::holmes::engine::types::Fact {
      pred_name : stringify!($pred_name).to_string(),
      args : vec![$(::holmes::pg::dyn::values::ToValue::to_value($a)),*]
    })
  };
  ($pred_name:ident($($a:expr),*)) => { |holmes: &mut ::holmes::Engine| {
    fact!(holmes, $pred_name($($a),*))
  }};
}

#[macro_export]
macro_rules! clause {
    ($holmes:ident, $vars:ident, $next:ident, $pred_name:ident($($m:tt),*)) => {{
        ::holmes::engine::types::Clause {
            pred_name: stringify!($pred_name).to_string(),
            args: vec![$(clause_match!($vars, $next, $m)),*]
        }
    }};
    ($holmes:ident, $vars:ident, $next:ident, $pred_name:ident{$($field:ident = $m:tt),*}) => {{
        use std::collections::HashMap;
        let pred_name = stringify!($pred_name).to_string();
        let pred = $holmes.get_predicate(&pred_name).unwrap().unwrap();
        let mut matches = HashMap::new();
        let _ = {
          $(matches.insert(stringify!($field).to_string(), clause_match!($vars, $next, $m)));*
        };
        let args: Vec<_> = pred.fields.iter().map(|field| {
            match field.name {
                Some(ref name) => match matches.remove(name) {
                    Some(cm) => cm,
                    None => ::holmes::engine::types::MatchExpr::Unbound
                },
                None => ::holmes::engine::types::MatchExpr::Unbound,
            }
        }).collect();
        ::holmes::engine::types::Clause {
            pred_name: pred_name,
            args: args
        }
    }};
}

/// Runs a datalog query against the `Holmes` context
///
/// Matches as per the right hand side of a datalog rule, then returns
/// a list of possible assignments to variables.
///
/// Clauses are separated by `&`, slots follow the rules in `match_expr!`
///
/// ```c
/// query!(holmes, foo((3), [_]) & bar([_], x))
/// ```
#[macro_export]
macro_rules! query {
  ($holmes:ident, $($pred_name:ident $inner:tt)&*) => {{
    use std::collections::HashMap;
    let mut _vars : HashMap<String, ::holmes::engine::types::Var> = HashMap::new();
    let mut _n : ::holmes::engine::types::Var = 0;
    let query = vec![$(clause!($holmes, _vars, _n, $pred_name $inner)),*];
    $holmes.derive(&query)
  }}
}

/// Adds a Holmes rule to the system
///
/// # Datalog Rules
///
/// ```c
/// rule!(holmes, baz([x], (7)) <= foo((3), [_]) & bar([_], x))
/// ```
///
/// will work as per a normal datalog rule, matching on foo and bar, and
/// generating a baz using any solutions found.
///
/// # Extended Rules
///
/// Holmes rules can also have "where clauses" which call out to native code
/// in the event of a match. For example,
///
/// ```c
/// rule!(holmes, baz([y], (8)) <= foo((3), [_]) & bar([_], x), {
///   let y = {f(x)}
/// })
/// ```
///
/// would call the Holmes registered function `f` on each output of `x`, bind
/// the result to `y`, and output it in the first slot of `baz`.
///
/// For more information on the expression and bind syntax, see the `hexpr!`
/// and `bind_match!` macro docs.
#[macro_export]
macro_rules! rule {
  ($holmes:ident, $rule_name:ident : $head_name:ident $head_inner:tt <= $($body_name:ident $body_inner:tt)&*,
   {$(let $bind:tt = $hexpr:tt);*}) => {{
    use std::collections::HashMap;
    let mut _vars : HashMap<String, ::holmes::engine::types::Var> = HashMap::new();
    let mut _n : ::holmes::engine::types::Var = 0;
    let body = vec![$(clause!($holmes, _vars, _n, $body_name $body_inner)),*];
    let wheres = vec![$(::holmes::engine::types::WhereClause {
        lhs: bind_match!(_vars, _n, $bind),
        rhs: hexpr!(_vars, _n, $hexpr)
    }),*];
    let head = clause!($holmes, _vars, _n, $head_name $head_inner);
    $holmes.new_rule(&::holmes::engine::types::Rule {
      name: stringify!($rule_name).to_string(),
      body: body,
      head: head,
      wheres: wheres,
    })
  }};
  ($holmes:ident, $rule_name:ident : $($head_name:ident $head_inner:tt),* <= $($body_name:ident $inner:tt)&*) => {
      rule!($holmes, $rule_name : $($head_name $head_inner),* <= $($body_name $inner)&*, {})
  };
  ($rule_name:ident : $($head_name:ident $head_inner:tt),* <= $($body_name:ident $inner:tt)&*) => {
    |holmes: &mut ::holmes::Engine| {
      rule!(holmes, $rule_name : $($head_name $head_inner),* <= $($body_name $inner)&*, {})
    }
  };
  ($rule_name:ident : $($head_name:ident $head_inner:tt),* <=
   $($body_name:ident $inner:tt)&*, {$(let $bind:tt = $hexpr:tt);*}) => {
    |holmes: &mut ::holmes::Engine| {
      rule!(holmes, $rule_name : $($head_name $head_inner),* <=
                    $($body_name $inner)&*, {$(let $bind = $hexpr);*})
    }
  };

}

/// Registers a native rust function with the `Holmes` object for use in rules.
///
/// ```c
/// func!(holmes, let f : uint64 -> string = |x : &u64| {
///   format!("{}", x)
/// })
/// ```
///
/// If your function input has more than one parameter, they will be tupled
/// and packed into a value.
/// To describe such a function, just use a tuple type on the left of the
/// arrow.
#[macro_export]
macro_rules! func {
  ($holmes:ident, let $name:ident : $src:tt -> $dst:tt = $body:expr) => {{
    let src = htype!($holmes, $src);
    let dst = htype!($holmes, $dst);
    $holmes.reg_func(stringify!($name).to_string(),
                     ::holmes::engine::types::Func {
                       input_type: src,
                       output_type: dst,
                       run: Box::new(move |v : ::holmes::pg::dyn::Value| {
                       ::holmes::pg::dyn::values::ToValue::to_value($body(typed_unpack!(v, $src)))
                     })})
  }};
  (let $name:ident : $src:tt -> $dst:tt = $body:expr) => {
    |holmes: &mut ::holmes::Engine| {
      func!(holmes, let $name : $src -> $dst = $body)
    }
  };
}

pub mod internal {
    //! EDSL Support Code
    //! This module contains support code for the other macros which is not
    //! intended to be user facing, but which must be exported for the macros
    //! to work properly.
    //!
    //! Until more complete example code is provided at the top of the module,
    //! the documentation in here may be useful for understanding the EDSL
    //! structure.

    /// Given a value and a type it is believed to be, unpack it to the greatest
    /// extent possible (e.g. unpack through tupling and lists)
    #[macro_export]
    macro_rules! typed_unpack {
    ($val:expr, [$typ:tt]) => {
      $val.get().downcast_ref::<Vec<::holmes::pg::dyn::Value>>()
          .expect("Dynamic list unpack failed")
          .into_iter().map(|v| {
        typed_unpack!(v, $typ)
      }).collect::<Vec<_>>()
    };
    ($val:expr, ($($typ:tt),*)) => {{
      let mut pack = $val.get().downcast_ref::<Vec<::holmes::pg::dyn::Value>>()
                         .expect("Dynamic tuple unpack failed").into_iter();
      ($(typed_unpack!(pack.next().expect("Dynamic tuple too short"), $typ)),*)
    }};
    ($val:expr, $name:ident) => {
        $val.get().downcast_ref()
        .expect(concat!("Dynamic base type unpack failed for ",
                        stringify!($name)))
    };
  }
    /// Constructs a bind match outer object.
    ///
    /// Args:
    ///
    /// * `$vars:ident` is a mutable `HashMap` from variable name to
    ///   variable number, to be updated as more variables are created, or
    ///   referenced to re-use existing variable numberings.
    /// * `$n:ident` is a mutable Var, intended to be used as an allocator for
    ///   the next unused variable. It should have a value equal to the next
    ///   unallocated variable
    /// * The last parameter is the bind expression, it can be structured as:
    ///   * `[bind_expression]` -> do a list destructure/iteration, similar to
    ///     the list monad
    ///   * {bind_expression0, bind_expression1} -> do a tuple destructure
    ///   * a `clause_match!` compatible expression (see `clause_match` docs)
    #[macro_export]
    macro_rules! bind_match {
        ($vars:ident, $n:ident, [ $bm:tt ]) => {
            ::holmes::engine::types::BindExpr::Iterate(
                Box::new(bind_match!($vars, $n, $bm)))
        };
        ($vars:ident, $n:ident, {$($bm:tt),*}) => {
            ::holmes::engine::types::BindExpr::Destructure(
                vec![$(bind_match!($vars, $n, $bm)),*])
        };
        ($vars:ident, $n:ident, $cm:tt) => {{
            ::holmes::engine::types::BindExpr::Normal(
                clause_match!($vars, $n, $cm))
        }};
    }

    /// Generates an expression structure
    ///
    /// Args:
    ///
    /// * `$vars:ident` is a mutable `HashMap` from variable name to
    ///   variable number, to be updated as more variables are created, or
    ///   referenced to re-use existing variable numberings.
    /// * `$n:ident` is a mutable Var, intended to be used as an allocator for
    ///   the next unused variable. It should have a value equal to the next
    ///   unallocated variable
    /// * the expression to convert
    ///   * `[var]`
    ///   * `(val)`
    ///   * `{f(expr, expr, expr)}`
    #[macro_export]
    macro_rules! hexpr {
    ($vars:ident, $n:ident, [$hexpr_name:ident]) => {{
      match clause_match!($vars, $n, $hexpr_name) {
        ::holmes::engine::types::MatchExpr::Var(var_no) =>
            ::holmes::engine::types::Expr::Var(var_no),
        _ => panic!("clause_match! returned non-var for var input")
      }
    }};
    ($vars:ident, $n:ident, ($hexpr:expr)) => {
      ::holmes::engine::types::Expr::Val(
          ::holmes::pg::dyn::values::ToValue::to_value($hexpr))
    };
    ($vars:ident, $n:ident, {$hexpr_func:ident($($hexpr_arg:tt),*)}) => {
      ::holmes::engine::types::Expr::App(
          stringify!($hexpr_func).to_string(),
          vec![$(hexpr!($vars, $n, $hexpr_arg)),*])
    };
  }

    /// Generates a `MatchExpr` from a representation
    ///
    /// Args:
    ///
    /// * `$vars:ident` is a mutable `HashMap` from variable name to
    ///   variable number, to be updated as more variables are created, or
    ///   referenced to re-use existing variable numberings.
    /// * `$n:ident` is a mutable Var, intended to be used as an allocator for
    ///   the next unused variable. It should have a value equal to the next
    ///   unallocated variable
    /// * Clause representation:
    ///   * `[_]` -> unbound
    ///   * `(val)` -> constant match
    ///   * `x` -> variable bind
    #[macro_export]
    macro_rules! clause_match {
    ($vars:ident, $n:ident, [_]) => {{
        ::holmes::engine::types::MatchExpr::Unbound
    }};
    ($vars:ident, $n:ident, ($v:expr)) => {{
        ::holmes::engine::types::MatchExpr::Const(
            ::holmes::pg::dyn::values::ToValue::to_value($v))
    }};
    ($vars:ident, $n:ident, $m:ident) => {{
      use std::collections::hash_map::Entry::*;
      use ::holmes::engine::types::MatchExpr::*;
      match $vars.entry(stringify!($m).to_string()) {
        Occupied(entry) => Var(*entry.get()),
        Vacant(entry) => {
          $n = $n + 1;
          entry.insert($n - 1);
          Var($n - 1)
        }
      }
    }};
  }
}
