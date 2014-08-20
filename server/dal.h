#ifndef HOLMES_SERVER_DAL_H_
#define HOLMES_SERVER_DAL_H_

#include <vector>
#include <map>

#include <kj/common.h>

#include "holmes.capnp.h"

namespace holmes {

class DAL {
  public:
    typedef std::map<std::string, Holmes::Val::Reader> Context;
    struct FactAssignment {
      FactAssignment(){}
      FactAssignment(Context ctx, std::vector<Holmes::Fact::Reader> facts)
        : context(ctx)
        , facts(facts) {}
      Context context;
      std::vector<Holmes::Fact::Reader> facts;
      inline void combine(FactAssignment &x){
        context.insert(x.context.begin(), x.context.end());
        facts.insert(facts.begin(), x.facts.begin(), x.facts.end());
      }
    };
    virtual ~DAL(){}
    virtual void setFact(Holmes::Fact::Reader) = 0;
    virtual std::vector<FactAssignment> getFacts(
      Holmes::FactTemplate::Reader,
      Context ctx = Context()) = 0;
    virtual void clean() = 0;
    virtual bool isDirty() = 0;
};

}

#endif
