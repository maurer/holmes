#ifndef HOLMES_SERVER_FACT_UTIL_H_
#define HOLMES_SERVER_FACT_UTIL_H_

#include <algorithm>

#include <kj/common.h>

#include "holmes.capnp.h"
#include "dal.h"

#define COMPARE_X_Y_VAL(accessor) \
  if (x.get ## accessor ## Val() < y.get ## accessor ## Val()) { \
    return true; \
  } else if (x.get ## accessor ## Val() > y.get ## accessor ## Val()) { \
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
        case Holmes::Val::JSON_VAL:
          COMPARE_X_Y_VAL(Json);
        case Holmes::Val::STRING_VAL:
          COMPARE_X_Y_VAL(String);
        case Holmes::Val::ADDR_VAL:
          COMPARE_X_Y_VAL(Addr);
        case Holmes::Val::BLOB_VAL:
          if (dc(x.getBlobVal(), y.getBlobVal())) {
            return true;
          } else if (dc(y.getBlobVal(), x.getBlobVal())) {
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
