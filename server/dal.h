#include "holmes.capnp.h"
#include <atomic>
#include <vector>
#include <set>
#include <mutex>
#include <capnp/message.h>

using namespace capnp;
using namespace std;

class DAL {
  public:
    virtual void setFact(Holmes::Fact::Reader) = 0;
    virtual vector<Holmes::Fact::Reader> getFacts(Holmes::FactTemplate::Reader) = 0;
};

class MemDAL : public DAL {
  private:
    class FactComp {
      public:
        bool operator() (const Holmes::Fact::Reader& x, const Holmes::Fact::Reader& y) const {
          if (x.getFactName() < y.getFactName()) {
            return true;
          } else if (x.getFactName() > y.getFactName()) {
            return false;
          }
          auto xs = x.getArgs();
          auto ys = y.getArgs();
          for (auto i = 0; i < xs.size(); i++) {
            switch (xs[i].which()) {
              case Holmes::Val::STRING_VAL:
                if (xs[i].getStringVal() < ys[i].getStringVal()) {
                  return true;
                } else if (xs[i].getStringVal() > ys[i].getStringVal()) {
                  return false;
                }
                break;
              case Holmes::Val::ADDR_VAL:
                if (xs[i].getAddrVal() < ys[i].getAddrVal()) {
                  return true;
                } else if (xs[i].getAddrVal() > ys[i].getAddrVal()) {
                  return false;
                }
                break;
            }
          }
          return false;
        }
    };
    std::mutex mutex;
    set<Holmes::Fact::Reader, FactComp> facts;
    MallocMessageBuilder mm;
  public:
    void setFact(Holmes::Fact::Reader);
    vector<Holmes::Fact::Reader> getFacts(Holmes::FactTemplate::Reader);
};
