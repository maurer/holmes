#include "dal.h"
#include <assert.h>
#include <capnp/message.h>
#include <iostream>

bool checkTypes(List<Holmes::Val>::Reader vals, List<Holmes::ArgMode>::Reader modes) {
  auto itv = vals.begin();
  auto itm = modes.begin();
  for (; (itv != vals.end()) && (itm != modes.end()); ++itv, ++itm) {
    switch (itm->getArgType()) {
      case Holmes::ArgType::ADDR:
        if (itv->which() != Holmes::Val::ADDR_VAL) {
	  return false;
	}
	break;
      case Holmes::ArgType::STRING:
        if (itv->which() != Holmes::Val::STRING_VAL) {
          return false;
        }
        break;
      default:
        return false;
    }
  }
  if ((itv != vals.end()) || (itm != modes.end())) {
    return false;
  }
  return true;
}

void MemDAL::setFact(Holmes::Fact::Reader fact) {
  std::lock_guard<std::mutex> lock(mutex);
  auto tid = fact.getTypeId();
  assert(tid < factTypes.size());
  auto modes = factTypes[tid];
  assert(checkTypes(fact.getArgs(), modes.getModes()));
  MallocMessageBuilder *neverFree = new MallocMessageBuilder();
  neverFree->setRoot(fact);
  auto fs = neverFree->getRoot<Holmes::Fact>();
  facts.push_back(fs);
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

List<Holmes::Fact>::Builder MemDAL::getFacts(Holmes::FactTemplate::Reader query, Holmes::DeriveResults::Builder builder) {
  std::lock_guard<std::mutex> lock(mutex);
  auto resultIndex = 0;
  vector<Holmes::Fact::Reader> filtered_facts;
  for (auto f : facts) {
    if (query.getTypeId() != f.getTypeId()) {
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
	case Holmes::TemplateVal::BOUND_VAR:
	case Holmes::TemplateVal::UNBOUND:
	  break;
      }
    }
    if (matched) {
      filtered_facts.push_back(f);
    }
  };
  List<Holmes::Fact>::Builder resultBuilder = builder.initFacts(filtered_facts.size());
  for (auto fact : filtered_facts) {
    resultBuilder.setWithCaveats(resultIndex++, fact);
  }
  return resultBuilder;
}

uint32_t MemDAL::newFactType(Holmes::FactSig::Reader modes) {
  std::lock_guard<std::mutex> lock(mutex);
  auto tid = factTypes.size();
  MallocMessageBuilder *neverFree = new MallocMessageBuilder();
  neverFree->setRoot(modes);
  auto fs = neverFree->getRoot<Holmes::FactSig>();
  factTypes.push_back(fs);
  return tid;
}
