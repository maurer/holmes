#include <iostream>

#include <capnp/ez-rpc.h>

#include "glog.h"
#include "holmes.h"

namespace holmes {

using kj::Own;
using kj::mv;

kj::Promise<void> HolmesImpl::runAll(std::set<std::string> dirty) {
  DLOG(INFO) << "runAll() entry";
  std::set<std::string> empty;
  kj::Promise<std::set<std::string>> x = kj::Promise<std::set<std::string>>(empty);
  for (auto analyzer : analyzers) {
    x = analyzer->run(dal.get(), dirty).then([x = mv(x)] (std::set<std::string> i) mutable {return mv(x).then([i = i](std::set<std::string> k){
      k.insert(i.begin(), i.end());
      return k;
    });});
  }
  return x.then([&](std::set<std::string> dirty){
    if (!dirty.empty()) {
      DLOG(INFO) << "DAL dirty, runAll() recursing";
      return runAll(dirty);
    } else {
      DLOG(INFO) << "DAL clean, runAll() returning";
      return static_cast<kj::Promise<void>>(kj::READY_NOW);
    }});
}

kj::Promise<void> HolmesImpl::set(SetContext context) {
  DLOG(INFO) << "set()";
  std::set<std::string> dirty = dal->setFacts(context.getParams().getFacts());
  if (dirty.empty()) {
    return kj::READY_NOW;
  }
  return runAll(dirty);
}

kj::Promise<void> HolmesImpl::derive(DeriveContext context) {
  auto ctxs = dal->getFacts(context.getParams().getTarget());
  auto builder = context.getResults().initCtx(ctxs.size());
  auto dex = 0;
  for (auto&& ctx : ctxs) {
    auto innerBuilder = builder.init(dex++, ctx.size());
    auto dex2 = 0;
    for (auto&& asgn : ctx) {
      innerBuilder.setWithCaveats(dex2++, asgn);
    }
  }
  return kj::READY_NOW;
}

kj::Promise<void> HolmesImpl::analyzer(AnalyzerContext context) {
  auto params = context.getParams();
  DLOG(INFO) << "analyzer() " << std::string(params.getName());
  Analyzer* a = new Analyzer(params.getName(), params.getPremises(), params.getAnalysis());
  analyzers.push_back(a);
  std::set<std::string> empty;
  return a->run(dal.get(), empty).then([this](std::set<std::string> m){
    kj::Promise<void> x = kj::NEVER_DONE;
    if (!m.empty()) {
      return runAll(m).then([](){
        return static_cast<kj::Promise<void>>(kj::NEVER_DONE);});
    } else {
      return x;
  }});
}
kj::Promise<void> HolmesImpl::registerType(RegisterTypeContext context) {
  auto params = context.getParams();
  DLOG(INFO) << "registerType() " << std::string(params.getFactName());
  bool valid = dal->addType(std::string(params.getFactName()),
                            params.getArgTypes());
  context.getResults().setValid(valid);
  return kj::READY_NOW;
}

}
