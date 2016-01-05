@0xaaef86128cdda946;

using Cxx = import "/capnp/c++.capnp";
$Cxx.namespace("holmes");

interface Holmes {
  # Values
  struct Val {
    union {
      uint64 @0 :UInt64;
      string @1 :Text;
      blob   @2 :Data;
      list   @3 :List(Val);
    }
  }

  struct HType {
    union {
      uint64 @0 :Void;
      string @1 :Void;
      blob   @2 :Void;
      list   @3 :HType;
    }
  }

  # Variables
  using Var = UInt32;

  # Logical facts
  using PredName = Text;
  struct Fact {
    predicate @0 :PredName;
    args      @1 :List(Val);
  }

  struct BodyExpr {
    union {
      unbound @0 :Void;
      var     @1 :Var;
      const   @2 :Val;
    }
  }

  struct BodyClause {
    predicate @0 :PredName;
    args      @1 :List(BodyExpr);
  }

  struct FExpr {
    func @0 :Text;
    args @1 :List(Expr);
  }

  struct Expr {
    union {
      var @0 :Var;
      val @1 :Val;
      app @2 : FExpr;
    }
  }

  struct BindExpr {
    union {
      normal  @0 :BodyExpr;
      iterate @1 :BodyExpr;
    }
  }

  struct WhereClause {
    lhs @0 :List(BindExpr);
    rhs @1 :Expr;
  }

  struct Rule {
    head @0 :List(BodyClause);
    body @1 :List(BodyClause);
    where @2 :List(WhereClause);
  }

  interface HFunc {
    types @0 ()->(inputTypes  : List(HType),
                  outputTypes : List(HType));
    run @1 (args :List(Val)) -> (results :List(Val));
  }

  # Register a predicate
  newPredicate @0 (predName :PredName,
                   argTypes :List(HType));

  # Add a fact to the extensional database
  newFact @1 (fact :Fact);

  # Ask the server to search or expand the intensional database
  # searching for a set of facts that matches a body clause
  # Returns the list of satisfying assignments to the body clauses.
  derive @2 (query :List(BodyClause)) -> (ctx :List(List(Val)));

  # Add a rule to expand the intentional database
  newRule @3 (rule :Rule) -> ();

  # Register a new external function
  newFunc @4 (name :Text, func :HFunc);
}
