#ifndef HOLMES_SERVER_PGDAL_H_
#define HOLMES_SERVER_PGDAL_H_

#include "dal.h"

#include <vector>
#include <set>
#include <atomic>
#include <mutex>

#include <kj/common.h>
#include <capnp/message.h>

#include <pqxx/pqxx>

#include "holmes.capnp.h"

namespace holmes {

class PgDAL : public DAL {
  public:
    PgDAL() : conn() {initDB();}
    PgDAL(std::string connStr) : conn(connStr) {initDB();}
    std::set<std::string> setFacts(capnp::List<Holmes::Fact>::Reader);
    std::set<std::string> setFacts(std::vector<Holmes::Fact::Reader>);
    bool addType(std::string name,
                 capnp::List<Holmes::HType>::Reader argTypes);
  private:
    std::mutex mutex;
    pqxx::connection conn;
    void initDB();
    std::map<std::string, std::vector<Holmes::HType>> types;
    typedef capnp::MallocMessageBuilder MMB;
    void registerPrepared(std::string, size_t);
    KJ_DISALLOW_COPY(PgDAL);
};

}

#endif
