#include "dal.h"
#include <assert.h>

bool checkTypes(List<Holmes::Val>::Reader vals, List<Holmes::ArgMode>::Reader modes) {
  auto itv = vals.begin();
  auto itm = modes.begin();
  for (; (itv != vals.end()) && (itm != modes.end()); ++itv, ++itm) {
    switch (itm->getType().which()) {
      case Holmes::ArgMode::Type::ADDR:
        if (itv->which() != Holmes::Val::ADDR_VAL) {
	  return false;
	}
	break;
      case Holmes::ArgMode::Type::STRING:
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
  auto tid = fact.getTypeId();
  assert(tid < typeId);
  auto modes = factTypes[tid];
  assert(checkTypes(fact.getArgs(), modes));
  Holmes::FactTemplate::Builder ftb(0);
  return;
}

List<Holmes::Fact>::Builder MemDAL::getFacts(Holmes::FactTemplate::Reader query) {
  List<Holmes::Fact>::Builder resultBuilder(0);
  return resultBuilder;
}

uint32_t MemDAL::newFactType(List<Holmes::ArgMode>::Reader modes) {
  auto tid = typeId++;
  factTypes[tid] = modes;
  return tid;
}
