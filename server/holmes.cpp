#include <iostream>
#include "holmes.capnp.h"
#include <capnp/ez-rpc.h>
#include "dal.h"
#include <capnp/message.h>
#include <map>

using namespace std;
using namespace capnp;
using namespace kj;

class HolmesImpl final : public Holmes::Server {
  private:
    class Analyzer {
      MallocMessageBuilder premBuilder, analBuilder;
      List<Holmes::FactTemplate>::Reader premises;
      Holmes::Analysis::Client analysis;
      uint64_t cache;
      public:
        Promise<void> run(DAL& dal) {
          auto req = analysis.analyzeRequest();
          //TODO: Give facts to it here
          auto facts = req.send();
          return facts.then([&](Holmes::Analysis::AnalyzeResults::Reader res){
            auto dfs = res.getDerived();
            for (auto f : dfs) {
              dal.setFact(f);
            }
          });
        }
        Analyzer(List<Holmes::FactTemplate>::Reader oPremises, Holmes::Analysis::Client oAnalysis) :
        premises(List<Holmes::FactTemplate>::Reader(oPremises)),
        analysis(oAnalysis)
        {
          premBuilder.setRoot(oPremises);
          premBuilder.getRoot<List<Holmes::FactTemplate> >();
        }
    };
    MemDAL dal;
    vector<Analyzer*> analyzers;
  public:
    Promise<void> set(SetContext context) override {
      dal.setFact(context.getParams().getFact());
      return READY_NOW;
    }
    Promise<void> derive(DeriveContext context) override {
      //Trigger relevant analyses here
      //TODO:
      //Initially, we'll just trigger analyses, then check the db
      //Later, we'll want to support long running stuff, and a good way might
      //be:
      //1.) Add an optional parameter of a notification interface
      //2.) Return a pair of a fact list, and a continuation which will
      //    either respond with more facts and another continuation
      //    or say that there's no way to get more.
      //Interface #1 if present would get called when more was available on
      //the continuation.
      dal.getFacts(context.getParams().getTarget(), context.getResults());
      return READY_NOW;
    }
    Promise<void> analyzer(AnalyzerContext context) override {
      auto params = context.getParams();
      Analyzer* a = new Analyzer(params.getPremises(), params.getAnalysis());
      analyzers.push_back(a);
      return a->run(dal);
    }
    Promise<void> newFactType(NewFactTypeContext context) override {
      auto sig = context.getParams().getFactSig();
      context.getResults().setFreshFactTypeId(dal.newFactType(sig));
      return READY_NOW;
    }
};

int main(int argc, const char* argv[]) {
  EzRpcServer server("*");
  server.exportCap("holmes", heap<HolmesImpl>());

  auto &waitScope = server.getWaitScope();
  uint port = server.getPort().wait(waitScope);
  cout << "Listening on port " << port << endl;
  NEVER_DONE.wait(waitScope);
}
