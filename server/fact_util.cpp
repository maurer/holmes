#include "fact_util.h"
#include "glog.h"

namespace holmes {

bool type_eq(Holmes::HType::Reader a, Holmes::HType::Reader b) {
  if (a.which() != b.which()) {
    return false;
  }
  if (a.which() == Holmes::HType::LIST) {
    return type_eq(a.getList(), b.getList());
  }
  return true;
}

bool typecheck(const std::map<std::string, std::vector<Holmes::HType::Reader>> &types,
               Holmes::Fact::Reader fact) {
  auto itt = types.find(fact.getFactName());
  if (itt == types.end()) {
    LOG(ERROR) << "Fact not found: " << std::string(fact.getFactName());
    return false;
  }
  auto fa = fact.getArgs();
  auto ts = itt->second;
  if (fa.size() != ts.size()) {
    LOG(ERROR) << "Arity mismatch for fact " << std::string(fact.getFactName()) << ", expected " << ts.size() << " got " << fa.size();
    return false;
  }
  for (size_t i = 0; i < fa.size(); i++) {
    switch (fa[i].which()) {
      case Holmes::Val::JSON_VAL:
        if (ts[i].which() != Holmes::HType::JSON) {
          LOG(ERROR) << "Non-json value at position " << i << " in fact " << std::string(fact.getFactName());
          return false;
        }
        break;
      case Holmes::Val::STRING_VAL:
        if (ts[i].which() != Holmes::HType::STRING) {
          LOG(ERROR) << "Non-string value at position " << i << " in fact " << std::string(fact.getFactName());
          return false;
        }
        break;
      case Holmes::Val::ADDR_VAL:
        if (ts[i].which() != Holmes::HType::ADDR) {
          LOG(ERROR) << "Non-addr value at position " << i << " in fact " << std::string(fact.getFactName());
          return false;
        }
        break;
      case Holmes::Val::BLOB_VAL:
        if (ts[i].which() != Holmes::HType::BLOB) {
          LOG(ERROR) << "Non-blob value at position " << i << " in fact " << std::string(fact.getFactName());
          return false;
        }
        break;
      case Holmes::Val::LIST_VAL:
        if (ts[i].which() != Holmes::HType::LIST) {
          LOG(ERROR) << "Non-blob value at position " << i << " in fact " << std::string(fact.getFactName());
          return false;
        }
        //TODO check inner types here
        //This requires a slight refactor, so I'm delaying until the rest works
        break;
    }
  }
  return true;
}

}
