#include <iostream>

#include <capnp/ez-rpc.h>

#include "holmes.capnp.h"
#include "memDal.h"
#include "analyzer.h"

namespace holmes {

using kj::Own;
using kj::mv;

class HolmesImpl final : public Holmes::Server {
  private:
    Own<DAL> dal;
    std::vector<Analyzer*> analyzers;
    kj::Promise<void> runAll() {
      dal->clean();
      kj::Promise<void> x = kj::READY_NOW;
      for (auto analyzer : analyzers) {
        x = analyzer->run(dal.get()).then([x = mv(x)] () mutable {return mv(x);});
      }
      return x.then([&](){
        if (dal->isDirty()) {
          return runAll();
        } else {
          return static_cast<kj::Promise<void>>(kj::READY_NOW);
        }});
    }
  public:
    HolmesImpl(Own<DAL> dal) : dal(mv(dal)) {}
    kj::Promise<void> set(SetContext context) override {
      dal->setFact(context.getParams().getFact());
      return runAll();
    }
    kj::Promise<void> derive(DeriveContext context) override {
      auto factAssigns = dal->getFacts(context.getParams().getTarget());
      std::vector<Holmes::Fact::Reader> facts;
      for (auto factAssign : factAssigns) {
        facts.insert(facts.end(), factAssign.facts.begin(), factAssign.facts.end());
      }
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
      return a->run(dal.get()).then([](){kj::Promise<void> x = kj::NEVER_DONE; return x;});
    }
    kj::Promise<void> registerType(RegisterTypeContext context) override {
      auto params = context.getParams();
      bool valid = dal->addType(std::string(params.getFactName()),
                                params.getArgTypes());
      context.getResults().setValid(valid);
      return kj::READY_NOW;
    }
};

}

int main(int argc, const char* argv[]) {
  capnp::EzRpcServer server("*");
  kj::Own<holmes::DAL> base = kj::heap<holmes::MemDAL>();
  server.exportCap("holmes", kj::heap<holmes::HolmesImpl>(kj::mv(base)));

  auto &waitScope = server.getWaitScope();
  uint port = server.getPort().wait(waitScope);
  std::cout << port << std::endl;
  kj::NEVER_DONE.wait(waitScope);
}
