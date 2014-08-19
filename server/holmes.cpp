#include <iostream>

#include <capnp/ez-rpc.h>

#include "holmes.capnp.h"
#include "memDal.h"
#include "analyzer.h"

namespace holmes {

class HolmesImpl final : public Holmes::Server {
  private:
    MemDAL dal;
    std::vector<Analyzer*> analyzers;
    kj::Promise<void> runAll() {
      dal.clean();
      kj::Promise<void> x = kj::READY_NOW;
      for (auto analyzer : analyzers) {
        x = analyzer->run(dal).then([x = mv(x)] () mutable {return mv(x);});
      }
      return x.then([&](){
        if (dal.isDirty()) {
          return runAll();
        } else {
          return static_cast<kj::Promise<void>>(kj::READY_NOW);
        }});
    }
  public:
    kj::Promise<void> set(SetContext context) override {
      dal.setFact(context.getParams().getFact());
      return runAll();
    }
    kj::Promise<void> derive(DeriveContext context) override {
      auto facts = dal.getFacts(context.getParams().getTarget());
      auto builder = context.getResults().initFacts(facts.size());
      auto dex = 0;
      for (auto f : facts) {
        builder.setWithCaveats(dex++, f);
      }
      return kj::READY_NOW;
    }
    kj::Promise<void> analyzer(AnalyzerContext context) override {
      auto params = context.getParams();
      Analyzer* a = new Analyzer(params.getPremises(), params.getAnalysis());
      analyzers.push_back(a);
      return a->run(dal).then([](){kj::Promise<void> x = kj::NEVER_DONE; return x;});
    }
};

}

int main(int argc, const char* argv[]) {
  capnp::EzRpcServer server("*");
  server.exportCap("holmes", kj::heap<holmes::HolmesImpl>());

  auto &waitScope = server.getWaitScope();
  uint port = server.getPort().wait(waitScope);
  std::cout << "Listening on port " << port << std::endl;
  kj::NEVER_DONE.wait(waitScope);
}
