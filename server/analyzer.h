#ifndef HOLMES_SERVER_ANALYZER_H_
#define HOLMES_SERVER_ANALYZER_H_

#include <atomic>

#include <kj/async.h>
#include <kj/common.h>

#include <map>
#include <mutex>

#include "holmes.capnp.h"
#include "dal.h"
#include "fact_util.h"

#include <iostream>

namespace holmes {

class Analyzer {
  public:
    Analyzer(capnp::List<Holmes::FactTemplate>::Reader premises,
             Holmes::Analysis::Client analysis)
      : premises(premises)
      , analysis(analysis){} 
    kj::Promise<bool> run(DAL *dal);

  private:
    std::mutex callMutex;
    class Cache {
      private:
        std::mutex mutex;
        std::map<DAL::Context, size_t, ContextCompare> cache;
      public:
        bool miss(DAL::FactAssignment fa) {
          std::lock_guard<std::mutex> lock(mutex);
          auto iti = cache.find(fa.context);
          if (iti == cache.end()) {
            // No entry
            return true;
          } else {
            if (iti->second >= fa.facts.size()) {
              return false; //we've already seen these
            } else {
              return true; //We've seen this assignment, but new facts
            }
          }
        }
        void add(DAL::FactAssignment fa) {
          std::lock_guard<std::mutex> lock(mutex);
          auto iti = cache.find(fa.context);
          if (iti == cache.end()) {
            cache[fa.context] = fa.facts.size();
          } else {
            if (iti->second < fa.facts.size()) {
              iti->second = fa.facts.size();
            }
          }
        }
    };
    capnp::List<Holmes::FactTemplate>::Reader premises;
    Holmes::Analysis::Client analysis;
    Cache cache;
    KJ_DISALLOW_COPY(Analyzer);
};

}

#endif
