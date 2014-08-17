#include "memDal.h"

#include <iostream>

#include <capnp/message.h>

#include "fact_util.h"

namespace holmes {

void MemDAL::setFact(Holmes::Fact::Reader fact) {
  std::lock_guard<std::mutex> lock(mutex);
  if (facts.count(fact) != 0) {
    capnp::MallocMessageBuilder *builder = new capnp::MallocMessageBuilder();
    builder->setRoot(fact);
    facts.insert(builder->getRoot<Holmes::Fact>());
    mm.push_back(builder);
  }
}

std::vector<Holmes::Fact::Reader> MemDAL::getFacts(Holmes::FactTemplate::Reader query) {
  std::lock_guard<std::mutex> lock(mutex);
  auto resultIndex = 0;
  std::vector<Holmes::Fact::Reader> filtered_facts;
  for (auto f : facts) {
    if (query.getFactName() != f.getFactName()) {
      continue;
    }
    auto fa  = f.getArgs();
    auto qa  = query.getArgs();
    auto itf = fa.begin();
    auto itq = qa.begin();
    bool matched = true;
    for (; matched && (itf != fa.end()) && (itq != qa.end()); ++itf, ++itq) {
      switch (itq->which()) {
        case Holmes::TemplateVal::EXACT_VAL:
          ValCompare compare;
	  matched &= ~ (compare(itq->getExactVal(), *itf)
                     || compare(*itf, itq->getExactVal()));
          break;
	case Holmes::TemplateVal::UNBOUND:
	  break;
      }
    }
    if (matched) {
      filtered_facts.push_back(f);
    }
  };
  return filtered_facts;
}

}
