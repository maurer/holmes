@0xaaef86128cdda946;
using Cxx = import "/capnp/c++.capnp";

$Cxx.namespace("holmes");

interface Holmes {
  # Dynamic value type for use in facts
  struct Val {
    union {
      stringVal @0 :Text;
      addrVal   @1 :UInt64;
      blobVal   @2 :Data;
    }
  }
  
  # Type of a dynamic variable
  enum HType {
    string @0;
    addr   @1;
    blob   @2;
  }

  # Variables
  # There is probably a good way to use numbers for space/speed, but
  # for now ease of debugging is more important
  using Var = Text;

  # Assignments
  # Used in instanced analyses/queries
  struct Asgn {
    var @0 :Var;
    val @1 :Val;
  }

  # Logical facts
  using FactName = Text;
  struct Fact {
    factName @0 :FactName;
    args     @1 :List(Val);
  }

  # Argument restriction when searching
  struct TemplateVal {
    union {
      exactVal @0 :Val;  #Argument must have this exact value
      unbound  @1 :Void; #Argument is unrestricted
      bound    @2 :Var;  #Argument is bound to a var and must be consistent
    }
  }

  # FactTemplate to be used as a search query
  struct FactTemplate {
    factName @0 :FactName;
    args     @1 :List(TemplateVal);
  }
  
  # Callback provided by an analysis
  interface Analysis {
    analyze @0 (context :List(Asgn), premises :List(Fact)) -> (derived :List(Fact));
  }

  # Assert a fact to the server
  set @0 (fact :Fact);
  
  # Ask the server to search for facts
  # For now, variables don't do much (unless you want to unify two fields
  # of the same fact), but once search queries are more complex we may
  # return an Asgn structure.
  derive @1 (target :FactTemplate) -> (facts :List(Fact));
  
  # Register as an analysis
  analyzer @2 (name        :Text,
               premises    :List(FactTemplate),
	       analysis    :Analysis);

  # Register a fact type
  # If it's not present, inform the DAL
  # If it is present, check compatibility
  registerType @3 (factName :Text,
                   argTypes :List(HType)) -> (valid :Bool);
}
