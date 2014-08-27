#include "fact_util.h"

namespace holmes {

bool typecheck(const std::map<std::string, std::vector<Holmes::HType>> &types,
               Holmes::Fact::Reader fact) {
  auto itt = types.find(fact.getFactName());
  if (itt == types.end()) {
    return false;
  }
  auto fa = fact.getArgs();
  auto ts = itt->second;
  if (fa.size() != ts.size()) {
    return false;
  }
  for (size_t i = 0; i < fa.size(); i++) {
    switch (fa[i].which()) {
      case Holmes::Val::STRING_VAL:
        if (ts[i] != Holmes::HType::STRING) {
          return false;
        }
        break;
      case Holmes::Val::ADDR_VAL:
        if (ts[i] != Holmes::HType::ADDR) {
          return false;
        }
        break;
      case Holmes::Val::BLOB_VAL:
        if (ts[i] != Holmes::HType::BLOB) {
          return false;
        }
        break;
    }
  }
  return true;
}

}
