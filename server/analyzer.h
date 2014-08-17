#ifndef HOLMES_SERVER_ANALYZER_H_
#define HOLMES_SERVER_ANALYZER_H_

#include <atomic>

#include <kj/async.h>
#include <kj/common.h>

#include "holmes.capnp.h"

namespace holmes {

class DAL;

class Analyzer {
  public:
    Analyzer(capnp::List<Holmes::FactTemplate>::Reader premises,
             Holmes::Analysis::Client analysis)
      : premises(premises)
      , analysis(analysis)
      , cache(0){}
    kj::Promise<void> run(DAL& dal);

  private:
    capnp::List<Holmes::FactTemplate>::Reader premises;
    Holmes::Analysis::Client analysis;
    std::atomic<uint64_t> cache;
    KJ_DISALLOW_COPY(Analyzer);
};

}

#endif
