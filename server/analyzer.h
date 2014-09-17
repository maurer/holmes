#ifndef HOLMES_SERVER_ANALYZER_H_
#define HOLMES_SERVER_ANALYZER_H_

#include <atomic>

#include <kj/async.h>
#include <kj/common.h>

#include <map>
#include <set>
#include <mutex>

#include "holmes.capnp.h"
#include "dal.h"
#include "fact_util.h"

#include <iostream>

namespace holmes {

class Analyzer {
  public:
    Analyzer(std::string name,
             capnp::List<Holmes::FactTemplate>::Reader premises,
             Holmes::Analysis::Client analysis)
      : name(name)
      , premises(premises)
      , analysis(analysis){
      for (auto&& premise : premises) {
        dependent.insert(premise.getFactName());
      }
    } 
    kj::Promise<std::set<std::string>> run(DAL *dal, std::set<std::string> dirty);

  private:
    std::string name;
    std::mutex callMutex;
    class Cache {
      private:
        std::mutex mutex;
        std::map<DAL::Context, size_t, ContextCompare> cache;
      public:
        bool miss(DAL::Context ctx) {
          std::lock_guard<std::mutex> lock(mutex);
          auto iti = cache.find(ctx);
          if (iti == cache.end()) {
            // No entry
            return true;
          } else {
            return false;
          }
        }
        void add(DAL::Context ctx) {
          std::lock_guard<std::mutex> lock(mutex);
          auto iti = cache.find(ctx);
          if (iti == cache.end()) {
            cache[ctx] = 1; //Leaving as a map, this will be where
                                   //we add forall
          } else {
            iti->second = 1;
          }
        }
    };
    std::set<std::string> dependent;
    capnp::List<Holmes::FactTemplate>::Reader premises;
    Holmes::Analysis::Client analysis;
    Cache cache;
    KJ_DISALLOW_COPY(Analyzer);
};

}

#endif
