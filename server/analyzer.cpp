#include "analyzer.h"

#include "dal.h"

#include "glog.h"
#include <iostream>

namespace holmes {

int n;

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
  std::vector<DAL::Context*> uncached;
  for (auto&& ctx : ctxs) {
    if (cache.miss(ctx)) {
      uncached.push_back(&ctx);
      cache.add(ctx);
    }
  }
  kj::Array<kj::Promise<capnp::Response<Holmes::Analysis::AnalyzeResults>>> analResults =
    KJ_MAP(ctx, uncached) {
      auto req = analysis.analyzeRequest();
      auto ctxBuilder = req.initContext(ctx->size());
      auto dex = 0;
      for (auto&& val : *ctx) {
        ctxBuilder.setWithCaveats(dex++, val);
      }
      DLOG(INFO) << "Sending request " << n << " for " << name;
      return (kj::Promise<capnp::Response<Holmes::Analysis::AnalyzeResults>>)req.send();
    };
  return kj::joinPromises(kj::mv(analResults)).then([this, dal](kj::Array<capnp::Response<Holmes::Analysis::AnalyzeResults>> analResults){
    std::vector<Holmes::Fact::Reader> insertion;
    for (auto&& x : analResults) {
      auto dfs = x.getDerived();
      for (auto&& df : dfs) {
        insertion.push_back(df);
      }
    }
    DLOG(INFO) << "Facts aggregated, submitting to db.";
    auto q = dal->setFacts(insertion);
    DLOG(INFO) << "Facts submitted.";
    return q;
  });
}

}
