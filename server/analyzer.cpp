#include "analyzer.h"

#include "dal.h"

#include "glog.h"
#include <iostream>

namespace holmes {

kj::Promise<bool> Analyzer::run(DAL *dal) {
  std::vector<Holmes::Fact::Reader> searchedFacts;
  DLOG(INFO) << "Starting analysis " << name;
  DLOG(INFO) << "Getting facts for " << name;
  auto ctxs = dal->getFacts(premises);
  DLOG(INFO) << "Got facts for " << name;
  kj::Array<kj::Promise<bool>> analResults =
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
          bool dirty = false;
          if (dal->setFacts(dfs) != 0) {
            dirty = true;
          }
          cache.add(ctx);
          return dirty;
        });
      }
      return kj::Promise<bool>(false);
    };
  return kj::joinPromises(kj::mv(analResults)).then([this](kj::Array<bool> x){
    bool dirty = false;
    for (auto v : x) {
      dirty |= v;
    }
    DLOG(INFO) << "Finished analysis " << name;
    return dirty;
  });
}

}
