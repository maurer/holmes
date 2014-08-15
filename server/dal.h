#include "holmes.capnp.h"
#include <atomic>
#include <vector>
#include <mutex>
#include <capnp/message.h>

using namespace capnp;
using namespace std;

class DAL {
  virtual void setFact(Holmes::Fact::Reader) = 0;
  virtual List<Holmes::Fact>::Builder getFacts(Holmes::FactTemplate::Reader, Holmes::DeriveResults::Builder) = 0;
  virtual uint32_t newFactType(Holmes::FactSig::Reader) = 0;
  //sync/export features?
};

class MemDAL : DAL {
  private:
    vector<Holmes::FactSig::Reader> factTypes;
    std::mutex mutex;
    vector<Holmes::Fact::Reader> facts;
    MallocMessageBuilder mm;
  public:
    void setFact(Holmes::Fact::Reader);
    List<Holmes::Fact>::Builder getFacts(Holmes::FactTemplate::Reader, Holmes::DeriveResults::Builder);
    uint32_t newFactType(Holmes::FactSig::Reader);
};
