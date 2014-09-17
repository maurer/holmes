#include "analyzer.h"

#include "dal.h"

#include "glog.h"
#include <iostream>

namespace holmes {

kj::Promise<std::set<std::string>> Analyzer::run(DAL *dal, std::set<std::string> olddirty) {
  if (!olddirty.empty()) {
    std::vector<std::string> intersect;
    std::set_intersection(dependent.begin(), dependent.end(), olddirty.begin(), olddirty.end(), back_inserter(intersect));
    if (intersect.empty()) {
      //None of our stuff was updated, this is none of our business
      std::set<std::string> empty;
      return kj::Promise<std::set<std::string>>(empty);
    }
  }
  std::vector<Holmes::Fact::Reader> searchedFacts;
  DLOG(INFO) << "Starting analysis " << name;
  DLOG(INFO) << "Getting facts for " << name;
  auto ctxs = dal->getFacts(premises);
  DLOG(INFO) << "Got facts for " << name;
  kj::Array<kj::Promise<std::set<std::string>>> analResults =
    KJ_MAP(ctx, ctxs) {
      if (cache.miss(ctx)) {
        auto req = analysis.analyzeRequest();
        auto ctxBuilder = req.initContext(ctx.size());
        auto dex = 0;
        for (auto&& val : ctx) {
          ctxBuilder.setWithCaveats(dex++, val);
        }
        return req.send().then([this, dal, ctx = kj::mv(ctx)](Holmes::Analysis::AnalyzeResults::Reader res){
          auto dfs = res.getDerived();
          std::set<std::string> dirty = dal->setFacts(dfs);
          cache.add(ctx);
          return dirty;
        });
      }
      std::set<std::string> empty;
      return kj::Promise<std::set<std::string>>(empty);
    };
  return kj::joinPromises(kj::mv(analResults)).then([this](kj::Array<std::set<std::string>> x){
    std::set<std::string> dirty;
    for (auto v : x) {
      dirty.insert(v.begin(), v.end());
    }
    DLOG(INFO) << "Finished analysis " << name;
    return dirty;
  });
}

}
