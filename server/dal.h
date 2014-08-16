#include "holmes.capnp.h"
#include <atomic>
#include <vector>
#include <mutex>
#include <capnp/message.h>

using namespace capnp;
using namespace std;

class DAL {
  public:
  virtual void setFact(Holmes::Fact::Reader) = 0;
  virtual vector<Holmes::Fact::Reader> getFacts(Holmes::FactTemplate::Reader) = 0;
  virtual uint32_t newFactType(Holmes::FactSig::Reader) = 0;
  //sync/export features?
};

class MemDAL : public DAL {
  private:
    vector<Holmes::FactSig::Reader> factTypes;
    std::mutex mutex;
    vector<Holmes::Fact::Reader> facts;
    MallocMessageBuilder mm;
  public:
    void setFact(Holmes::Fact::Reader);
    vector<Holmes::Fact::Reader> getFacts(Holmes::FactTemplate::Reader);
    uint32_t newFactType(Holmes::FactSig::Reader);
};
