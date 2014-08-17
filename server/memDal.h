#ifndef HOLMES_SERVER_MEMDAL_H_
#define HOLMES_SERVER_MEMDAL_H_

#include "dal.h"

#include <vector>
#include <set>
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
    std::vector<Holmes::Fact::Reader> getFacts(Holmes::FactTemplate::Reader);

  private:
    std::mutex mutex;
    std::set<Holmes::Fact::Reader, FactCompare> facts;
    std::vector<capnp::MessageBuilder*> mm;
    KJ_DISALLOW_COPY(MemDAL);
};

}

#endif
