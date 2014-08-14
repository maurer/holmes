#include "holmes.capnp.h"
#include <atomic>
#include <vector>

using namespace capnp;
using namespace std;

class DAL {
  virtual void setFact(Holmes::Fact::Reader) = 0;
  virtual List<Holmes::Fact>::Builder getFacts(Holmes::FactTemplate::Reader) = 0;
  virtual uint32_t newFactType(List<Holmes::ArgMode>::Reader) = 0;
  //sync/export features?
};

class MemDAL : DAL {
  private:
    atomic<uint32_t> typeId;
    vector<List<Holmes::ArgMode>::Reader> factTypes;
  public:
    void setFact(Holmes::Fact::Reader);
    List<Holmes::Fact>::Builder getFacts(Holmes::FactTemplate::Reader);
    uint32_t newFactType(List<Holmes::ArgMode>::Reader);
};
