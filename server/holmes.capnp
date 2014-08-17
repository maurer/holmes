@0xaaef86128cdda946;

interface Holmes {
  # Dynamic value type for use in facts
  struct Val {
    union {
      stringVal @0 :Text;
      addrVal   @1 :UInt64;
    }
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
    }
  }

  # FactTemplate to be used as a search query
  struct FactTemplate {
    factName @0 :FactName;
    args     @1 :List(TemplateVal);
  }
  
  # Callback provided by an analysis
  interface Analysis {
    analyze @0 (premises :List(Fact)) -> (derived :List(Fact));
  }

  # Assert a fact to the server
  set @0 (fact :Fact);
  # Ask the server to search for facts
  derive @1 (target :FactTemplate) -> (facts :List(Fact));
  # Register as an analysis
  analyzer @2 (premises    :List(FactTemplate),
	       analysis    :Analysis);
}
