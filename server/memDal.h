#ifndef HOLMES_SERVER_MEMDAL_H_
#define HOLMES_SERVER_MEMDAL_H_

#include "dal.h"

#include <vector>
#include <set>
#include <atomic>
#include <mutex>

#include <kj/common.h>
#include <capnp/message.h>

#include "holmes.capnp.h"
#include "fact_util.h"

namespace holmes {

class MemDAL : public DAL {
  public:
    MemDAL(){}
    ~MemDAL() {
      for (auto b : mm) {
        delete b;
      }
    }
    void setFact(Holmes::Fact::Reader);
    std::vector<FactAssignment> getFacts(
      Holmes::FactTemplate::Reader,
      Context ctx = Context());
    void clean() {dirty = false;}
    bool isDirty() { return dirty; }

  private:
    std::mutex mutex;
    std::atomic<bool> dirty;
    std::set<Holmes::Fact::Reader, FactCompare> facts;
    std::vector<capnp::MessageBuilder*> mm;
    KJ_DISALLOW_COPY(MemDAL);
};

}

#endif
