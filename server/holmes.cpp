#include <iostream>

#include <capnp/ez-rpc.h>
#include <glog/logging.h>
#include <gflags/gflags.h>

#include "holmes.capnp.h"
#include "pgDal.h"
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
      DLOG(INFO) << "runAll() entry";
      kj::Promise<bool> x = kj::Promise<bool>(false);
      for (auto analyzer : analyzers) {
        x = analyzer->run(dal.get()).then([x = mv(x)] (bool i) mutable {return mv(x).then([i = i](bool k){return i || k;});});
      }
      return x.then([&](bool dirty){
        if (dirty) {
          DLOG(INFO) << "DAL dirty, runAll() recursing";
          return runAll();
        } else {
          DLOG(INFO) << "DAL clean, runAll() returning";
          return static_cast<kj::Promise<void>>(kj::READY_NOW);
        }});
    }
  public:
    HolmesImpl(Own<DAL> dal) : dal(mv(dal)) {}
    kj::Promise<void> set(SetContext context) override {
      DLOG(INFO) << "set()";
      if (dal->setFact(context.getParams().getFact())) {
        return runAll();
      }
      return kj::READY_NOW;
    }
    kj::Promise<void> derive(DeriveContext context) override {
      auto factAssigns = dal->getFacts(context.getParams().getTarget(), DAL::Context());
      std::vector<Holmes::Fact::Reader> facts;
      for (auto&& factAssign : factAssigns.results) {
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
      Analyzer* a = new Analyzer(params.getName(), params.getPremises(), params.getAnalysis());
      analyzers.push_back(a);
      return a->run(dal.get()).then([this](bool m){
        kj::Promise<void> x = kj::NEVER_DONE;
        if (m) {
          return runAll().then([](){
            return static_cast<kj::Promise<void>>(kj::NEVER_DONE);});
        } else {
          return x;
      }});
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

int main(int argc, char* argv[]) {
  gflags::ParseCommandLineFlags(&argc, &argv, true);
  google::InitGoogleLogging(argv[0]);
  
  capnp::EzRpcServer server("*");
  kj::Own<holmes::DAL> base = kj::heap<holmes::PgDAL>();
  server.exportCap("holmes", kj::heap<holmes::HolmesImpl>(kj::mv(base)));

  auto &waitScope = server.getWaitScope();
  uint port = server.getPort().wait(waitScope);
  LOG(INFO) << "Running on port: " << port;
  std::cout << port << std::endl;
  kj::NEVER_DONE.wait(waitScope);
}
