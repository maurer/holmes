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
    }
  }

  enum HType {
    uint64 @0;
    string @1;
    blob   @2;
  }

  # Variables
  using Var = UInt32;

  # Logical facts
  using PredName = Text;
  using PredId   = UInt64;
  struct Fact {
    predicate @0 :PredId;
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
    predicate @0 :PredId;
    args      @1 :List(BodyExpr);
  }

  struct Rule {
    head @0 :BodyClause;
    body @1 :List(BodyClause);
  }

  # Register a predicate
  registerPredicate @0 (predName :PredName,
                        argTypes :List(HType)) -> (predId :PredId);

  # Add a fact to the extensional database
  set @1 (fact :List(Fact));
  
  # Ask the server to search or expand the intensional database
  # searching for a set of facts that matches a body clause
  # Returns the list of satisfying assignments to the body clauses.
  derive @2 (target :List(BodyClause)) -> (ctx :List(List(Val)));

  # Add a rule to expand the intentional database
  addRule @3 (rule :Rule) -> ();
}
