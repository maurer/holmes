#include "analyzer.h"

#include "dal.h"

#include <glog/logging.h>

#include <iostream>

namespace holmes {

kj::Promise<bool> Analyzer::run(DAL *dal) {
  DLOG(INFO) << "Running analysis: " << name;
  std::vector<Holmes::Fact::Reader> searchedFacts;
  std::vector<DAL::FactAssignment> fas;
  fas.push_back(DAL::FactAssignment());
  for (auto premise : premises) {
    std::vector<DAL::FactAssignment> newFas;
    for (auto fa : fas) {
      auto resFas = dal->getFacts(premise, fa.context);
      for (auto&& newFa : resFas.results) {
        newFa.combine(fa);
        newFas.push_back(newFa);
      }
    }
    fas = newFas;
  }
  DLOG(INFO) << "Found " << fas.size() << " instances.";
  kj::Array<kj::Promise<bool>> analResults =
    KJ_MAP(fa, fas) {
      if (cache.miss(fa)) {
        DLOG(INFO) << "Cache miss";
        auto req = analysis.analyzeRequest();
        auto premBuilder = req.initPremises(fa.facts.size());
        auto dex = 0;
        for (auto f : fa.facts) {
          premBuilder.setWithCaveats(dex++, f);
        }
        auto ctxBuilder = req.initContext(fa.context.size());
        dex = 0;
        for (auto kv : fa.context) {
          ctxBuilder[dex].setVar(kv.first);
          ctxBuilder[dex++].setVal(kv.second);
        }
        return req.send().then([this, dal, fa = kj::mv(fa)](Holmes::Analysis::AnalyzeResults::Reader res){
          auto dfs = res.getDerived();
          bool dirty = false;
          for (auto f : dfs) {
            dirty |= dal->setFact(f);
          }
          cache.add(fa);
          return dirty;
        });
      } else {
        DLOG(INFO) << "Cache hit";
      }
      return kj::Promise<bool>(false);
    };
  return kj::joinPromises(kj::mv(analResults)).then([](kj::Array<bool> x){
    bool dirty = false;
    for (auto v : x) {
      dirty |= v;
    }
    return dirty;
  });
}

}
