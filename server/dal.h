#ifndef HOLMES_SERVER_DAL_H_
#define HOLMES_SERVER_DAL_H_

#include <vector>
#include <map>

#include <kj/common.h>
#include <capnp/message.h>

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
    class FactResults {
      public:
        std::vector<FactAssignment> results;
        std::vector<capnp::MallocMessageBuilder*> mbs;
        ~FactResults() {
          /*for (auto mb : mbs) {
            delete mb;
          }*/
        }
    };
    virtual ~DAL(){}
    virtual bool setFact(Holmes::Fact::Reader) = 0;
    virtual bool addType(std::string, capnp::List<Holmes::HType>::Reader) = 0;
    virtual FactResults getFacts(
      Holmes::FactTemplate::Reader,
      Context ctx = Context()) = 0;
};

}

#endif
