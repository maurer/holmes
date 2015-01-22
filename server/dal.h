#ifndef HOLMES_SERVER_DAL_H_
#define HOLMES_SERVER_DAL_H_

#include <vector>
#include <set>
#include <map>

#include <kj/common.h>
#include <capnp/message.h>

#include "glog.h"
#include "holmes.capnp.h"

namespace holmes {

class DAL {
  public:
    virtual ~DAL(){}
};

}

#endif
