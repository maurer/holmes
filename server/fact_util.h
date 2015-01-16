#ifndef HOLMES_SERVER_FACT_UTIL_H_
#define HOLMES_SERVER_FACT_UTIL_H_

#include <algorithm>

#include <kj/common.h>

#include "holmes.capnp.h"
#include "dal.h"

#define COMPARE_X_Y_VAL(accessor) \
  if (x.get ## accessor() < y.get ## accessor()) { \
    return true; \
  } else if (x.get ## accessor() > y.get ## accessor()) { \
    return false; \
  } \
  break;

namespace holmes {

class DataCompare {
  public:
    bool operator() (const capnp::Data::Reader& x,
                     const capnp::Data::Reader& y) const {
      if (x.size() < y.size()) {
        return true;
      } else if (x.size() > y.size()) {
        return false;
      }

      for (size_t i = 0; i < x.size(); i++) {
        if (x[i] < y[i]) {
          return true;
        } else if (x[i] > y[i]) {
          return false;
        }
      }

      return false;
    }
};

class ValCompare {
  public:
    bool operator() (const Holmes::Val::Reader& x,
                     const Holmes::Val::Reader& y) const {
      if (x.which() < y.which()) {
        return true;
      } else if (x.which() > y.which()) {
        return false;
      }

      DataCompare dc;

      switch (x.which()) {
        case Holmes::Val::STRING:
          COMPARE_X_Y_VAL(String);
        case Holmes::Val::UINT64:
          COMPARE_X_Y_VAL(Uint64);
        case Holmes::Val::BLOB:
          if (dc(x.getBlob(), y.getBlob())) {
            return true;
          } else if (dc(y.getBlob(), x.getBlob())) {
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
      for (uint32_t i = 0; i < xs.size(); i++) {
        if (compare(xs[i], ys[i])) {
          return true;
        } else if (compare(ys[i], xs[i])) {
          return false;
        }
      }
      return false;
    }
};

class ContextCompare {
  public:
    bool operator() (const DAL::Context& x, const DAL::Context& y) {
      return std::lexicographical_compare(x.begin(), x.end(),
                                          y.begin(), y.end(),
                                          ValCompare());
    }
};

bool typecheck(const std::map<std::string, std::vector<Holmes::HType>> &types,
               Holmes::Fact::Reader fact);


}

#endif
