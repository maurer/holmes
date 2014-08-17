#include "analyzer.h"

#include "dal.h"

namespace holmes {

kj::Promise<void> Analyzer::run(DAL& dal) {
  auto req = analysis.analyzeRequest();
  std::vector<Holmes::Fact::Reader> searchedFacts;
  for (auto premise : premises) {
    auto premFacts = dal.getFacts(premise);
    std::copy(premFacts.begin(), premFacts.end(), std::back_inserter(searchedFacts));
  }
  uint64_t expected = cache;
  while (expected < searchedFacts.size()) {
    if (cache.compare_exchange_weak(expected, searchedFacts.size())) {
      auto premBuilder = req.initPremises(searchedFacts.size());
      auto dex = 0;
      for (auto f : searchedFacts) {
        premBuilder.setWithCaveats(dex++, f);
      }
      auto facts = req.send();
      return facts.then([&](Holmes::Analysis::AnalyzeResults::Reader res){
        auto dfs = res.getDerived();
        for (auto f : dfs) {
          dal.setFact(f);
        }
      });
    }
  }
  return kj::READY_NOW;
}

}
