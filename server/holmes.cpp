#include <iostream>

#include "glog.h"
#include "holmes.h"

namespace holmes {

using kj::Own;
using kj::mv;

kj::Promise<void> HolmesImpl::registerPredicate(RegisterPredicateContext context) {
  DLOG(INFO) << "registerType()";
  return kj::READY_NOW;
}

kj::Promise<void> HolmesImpl::set(SetContext context) {
  DLOG(INFO) << "set()";
  return kj::READY_NOW;
}

kj::Promise<void> HolmesImpl::derive(DeriveContext context) {
  return kj::READY_NOW;
}

kj::Promise<void> HolmesImpl::addRule(AddRuleContext context) {
  return kj::READY_NOW;
}

}
