#include "dal.h"
#include <assert.h>
#include <capnp/message.h>
#include <iostream>

void MemDAL::setFact(Holmes::Fact::Reader fact) {
  std::lock_guard<std::mutex> lock(mutex);
  MallocMessageBuilder *neverFree = new MallocMessageBuilder();
  neverFree->setRoot(fact);
  auto fs = neverFree->getRoot<Holmes::Fact>();
  facts.insert(fs);
}

bool eq_val(Holmes::Val::Reader x, Holmes::Val::Reader y) {
  if (x.which() != y.which()) {
    return false;
  }
  switch (x.which()) {
    case Holmes::Val::STRING_VAL:
      return (x.getStringVal() == y.getStringVal());
    case Holmes::Val::ADDR_VAL:
      return (x.getAddrVal() == y.getAddrVal());
  }
}

vector<Holmes::Fact::Reader> MemDAL::getFacts(Holmes::FactTemplate::Reader query) {
  std::lock_guard<std::mutex> lock(mutex);
  auto resultIndex = 0;
  vector<Holmes::Fact::Reader> filtered_facts;
  for (auto f : facts) {
    if (query.getFactName() != f.getFactName()) {
      continue;
    }
    auto fa  = f.getArgs();
    auto qa  = query.getArgs();
    auto itf = fa.begin();
    auto itq = qa.begin();
    bool matched = true;
    for (; (itf != fa.end()) && (itq != qa.end()); ++itf, ++itq) {
      switch (itq->which()) {
        case Holmes::TemplateVal::EXACT_VAL:
	  matched &= eq_val(itq->getExactVal(), *itf);
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
