#include "holmes.capnp.h"
#include "dal.h"

namespace holmes {

using kj::Own;

class HolmesImpl final : public Holmes::Server {
  private:
    Own<DAL> dal;
  public:
    HolmesImpl(Own<DAL> dal) : dal(mv(dal)) {}
    kj::Promise<void> registerPredicate(RegisterPredicateContext context) override;
    kj::Promise<void> set(SetContext context) override;
    kj::Promise<void> derive(DeriveContext context) override;
    kj::Promise<void> addRule(AddRuleContext context) override;
};
}
