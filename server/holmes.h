#include "holmes.capnp.h"
#include "analyzer.h"
#include "dal.h"

namespace holmes {

using kj::Own;

class HolmesImpl final : public Holmes::Server {
  private:
    Own<DAL> dal;
    std::vector<Analyzer*> analyzers;
    kj::Promise<void> runAll(std::set<std::string> dirty);
  public:
    HolmesImpl(Own<DAL> dal) : dal(mv(dal)) {}
    kj::Promise<void> set(SetContext context) override;
    kj::Promise<void> derive(DeriveContext context) override;
    kj::Promise<void> analyzer(AnalyzerContext context) override;
    kj::Promise<void> registerType(RegisterTypeContext context) override;
};

}
