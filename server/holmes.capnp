@0xaaef86128cdda946;

interface Holmes {
  # Assert a fact to the server
  set @0 (fact :Fact);
  # Ask the server to search for facts
  derive @1 (target :FactTemplate) -> (facts :List(Fact));
  # Register as an analysis
  analyzer @2 (premises    :List(FactTemplate),
               conclusions :List(FactTemplate),
	       analysis    :Analysis);
  # Register a new fact type
  # Arity/join/etc go here eventually
  using FactTypeId = UInt32;
  newFactType @3 (factSig :List(ArgMode)) -> (freshFactTypeId :FactTypeId);
  
  interface Analysis {
    analyze @0 (ctx :List(Asgn)) -> (derived :List(Fact));
  }

  struct ArgMode {
    type :union {
      string @0 :Void;
      addr   @1 :Void;
    }
    mode :union {
      equal  @2 :Void;
      ignore @3 :Void;
    }
  }

  struct Val {
    union {
      stringVal @0 :Text;
      addrVal   @1 :UInt64;
      # This will need to be expanded or made dynamic
    }
  }

  # Doing this now for easier debugging. In the long term, we probably want to use UInt32
  using Var = Text;
  
  struct Asgn {
    var @0 :Var;
    val @1 :Val;
  }

  struct TemplateVal {
    union {
      exactVal @0 :Val;
      boundVar @1 :Var;
      unbound  @2 :Void;
    }
  }

  struct Fact {
    typeId @0 :FactTypeId;
    args   @1 :List(Val);
  }

  struct FactTemplate {
    typeId @0 :FactTypeId;
    args   @1 :List(TemplateVal);
  }
}
