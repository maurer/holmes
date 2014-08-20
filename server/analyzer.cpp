#include "analyzer.h"

#include "dal.h"

namespace holmes {

kj::Promise<void> Analyzer::run(DAL& dal) {
  auto req = analysis.analyzeRequest();
  std::vector<Holmes::Fact::Reader> searchedFacts;
  std::vector<DAL::FactAssignment> fas;
  fas.push_back(DAL::FactAssignment());
  for (auto premise : premises) {
    std::vector<DAL::FactAssignment> newFas;
    for (auto fa : fas) {
      auto resFas = dal.getFacts(premise, fa.context);
      for (auto newFa : resFas) {
        newFa.combine(fa);
        newFas.push_back(newFa);
      }
    }
    fas = newFas;
  }
  kj::Array<kj::Promise<int>> analResults =
    KJ_MAP(fa, fas) {
      if (cache.miss(fa)) {
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
        return req.send().then([&, fa = kj::mv(fa)](Holmes::Analysis::AnalyzeResults::Reader res){
          auto dfs = res.getDerived();
          for (auto f : dfs) {
            dal.setFact(f);
          }
          cache.add(fa);
          return 0;
        });
      }
      return kj::Promise<int>(0);
    };
  return kj::joinPromises(kj::mv(analResults)).then([](kj::Array<int> x){});
}

}
