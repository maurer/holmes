#include "memDal.h"

#include <stack>

#include <capnp/message.h>

#include "fact_util.h"

namespace holmes {

void MemDAL::setFact(Holmes::Fact::Reader fact) {
  std::lock_guard<std::mutex> lock(mutex);
  if (facts.count(fact) == 0) {
    capnp::MallocMessageBuilder *builder = new capnp::MallocMessageBuilder();
    builder->setRoot(fact);
    facts.insert(builder->getRoot<Holmes::Fact>());
    mm.push_back(builder);
  }
  dirty = true;
}

std::vector<DAL::FactAssignment> MemDAL::getFacts(Holmes::FactTemplate::Reader query, Context ctx) {
  std::lock_guard<std::mutex> lock(mutex);
  std::map<Context, std::vector<Holmes::Fact::Reader>, ContextCompare> fam;

  for (auto f : facts) {
    if (query.getFactName() != f.getFactName()) {
      continue;
    }
    auto fa  = f.getArgs();
    auto qa  = query.getArgs();
    auto itf = fa.begin();
    auto itq = qa.begin();
    Context newCtx = ctx;
    bool matched = true;
    for (; matched && (itf != fa.end()) && (itq != qa.end()); ++itf, ++itq) {
      switch (itq->which()) {
        case Holmes::TemplateVal::EXACT_VAL:
          ValCompare compare;
	  matched &= ~ (compare(itq->getExactVal(), *itf)
                     || compare(*itf, itq->getExactVal()));
          break;
        case Holmes::TemplateVal::BOUND:
          {
            std::string var = itq->getBound();
            auto itv = newCtx.find(var);
            if (itv != newCtx.end()) {
              //Variable is already bound, check that it matches
              ValCompare compare;
              matched &= ~ (compare(itv->second, *itf)
                         || compare(*itf, itv->second));
            } else {
              //Variable is unbound, bind it
              newCtx.insert(std::pair<std::string, Holmes::Val::Reader>(var, *itf));
            }
          }
          break;
	case Holmes::TemplateVal::UNBOUND:
	  break;
      }
    }
    if (matched) {
      auto itc = fam.find(newCtx);
      if (itc != fam.end()) {
        itc->second.push_back(f);
      } else {
        fam[newCtx] = {f};
      }
    }
  }
  std::vector<FactAssignment> fas;
  for (auto fa : fam) {
    fas.push_back(FactAssignment(fa.first, fa.second));
  }
  return fas;
}

}
