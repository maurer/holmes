#ifndef HOLMES_SERVER_FACT_UTIL_H_
#define HOLMES_SERVER_FACT_UTIL_H_

#include <kj/common.h>

#include "holmes.capnp.h"

namespace holmes {

class ValCompare {
  public:
    bool operator() (const Holmes::Val::Reader& x,
                     const Holmes::Val::Reader& y) const {
      if (x.which() < y.which()) {
        return true;
      } else if (x.which() > y.which()) {
        return false;
      }

      switch (x.which()) {
        case Holmes::Val::STRING_VAL:
          if (x.getStringVal() < y.getStringVal()) {
            return true;
          } else if (x.getStringVal() > y.getStringVal()) {
            return false;
          }
          break;

        case Holmes::Val::ADDR_VAL:
          if (x.getAddrVal() < y.getAddrVal()) {
            return true;
          } else if (x.getAddrVal() > y.getAddrVal()) {
            return false;
          }
          break;
      }
      return false;
    }
};

class FactCompare {
  public:
    bool operator() (const Holmes::Fact::Reader& x,
                     const Holmes::Fact::Reader& y) const {
      if (x.getFactName() < y.getFactName()) {
        return true;
      } else if (x.getFactName() > y.getFactName()) {
        return false;
      }

      auto xs = x.getArgs();
      auto ys = y.getArgs();
      ValCompare compare;
      for (auto i = 0; i < xs.size(); i++) {
        if (compare(xs[i], ys[i])) {
          return true;
        } else if (compare(ys[i], xs[i])) {
          return false;
        }
      }
      return false;
    }
};

}

#endif
