#ifndef HOLMES_SERVER_DAL_H_
#define HOLMES_SERVER_DAL_H_

#include <vector>

#include <kj/common.h>

#include "holmes.capnp.h"

namespace holmes {

class DAL {
  public:
    virtual ~DAL(){}
    virtual void setFact(Holmes::Fact::Reader) = 0;
    virtual std::vector<Holmes::Fact::Reader> getFacts(Holmes::FactTemplate::Reader) = 0;
    virtual void clean() = 0;
    virtual bool isDirty() = 0;
};

}

#endif
