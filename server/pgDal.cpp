#include "pgDal.h"

#include <capnp/message.h>
#include <capnp/pretty-print.h>
#include <kj/debug.h>

#include <iostream>

#include <assert.h>

namespace holmes {

void PgDAL::initDB() {
  pqxx::work work(conn);
  work.exec("create schema if not exists facts");
  auto res = work.exec("select table_name, udt_name from information_schema.columns where table_schema = 'facts' ORDER BY table_name, ordinal_position");
  work.commit();
  for (auto line : res) {
    std::string name = line[0].c_str();
    if (types.find(name) == types.end()) {
      std::vector<Holmes::HType> sig;
      types[name] = sig;
    }
    auto typ = Holmes::HType::UINT64;
    std::string type_string = line[1].c_str();
    if (type_string == "int8") {
      typ = Holmes::HType::UINT64;
    } else if (type_string == "varchar") {
      typ = Holmes::HType::STRING;
    } else if (type_string == "bytea") {
      typ = Holmes::HType::BLOB;
    } else {
      std::cerr << "Type parse failure: " << type_string << std::endl;
      exit(1);
    }
    types[name].push_back(typ);
  }
  for (auto type : types) {
    registerPrepared(type.first, type.second.size());
  }
}

void PgDAL::registerPrepared(std::string name, size_t n) {
  std::string argVals = "(";
  for (size_t i = 1; i <= n; i++) {
    argVals += "$" + std::to_string(i);
    if (i != n) {
      argVals += ", ";
    } else {
      argVals += ")";
    }
  }
  conn.prepare(name + ".insert", "INSERT INTO facts." + name + " VALUES " + argVals);
}

std::set<std::string> PgDAL::setFacts(std::vector<Holmes::Fact::Reader> facts) {
  std::set<std::string> empty;
  return empty;
}

std::set<std::string> PgDAL::setFacts(capnp::List<Holmes::Fact>::Reader facts) {
  std::set<std::string> empty;
  return empty;
}


std::string htype_to_sqltype(Holmes::HType hType) {
  switch (hType) {
    case Holmes::HType::STRING:
      return "varchar";
    case Holmes::HType::UINT64:
      return "bigint";
    case Holmes::HType::BLOB:
      return "bytea";
  }
  return "unknown";
}

bool valid_name(std::string s) {
  for (auto c : s) {
    if (c == '_') {
      continue;
    }
    if ((c >= 'a') && (c <= 'z')) {
      continue;
    }
    if ((c >= '0') && (c <= '9')) {
      continue;
    }
    return false;
  }
  return true;
}

bool PgDAL::addType(std::string name, capnp::List<Holmes::HType>::Reader argTypes) {
  std::lock_guard<std::mutex> lock(mutex);
  //We're using this for a table name, so we have restrictions
  if (!valid_name(name)) {
    return false;
  }
  auto itt = types.find(name);
  if (itt != types.end()) {
    if (argTypes.size() != itt->second.size()) {
      return false;
    }
    for (size_t i = 0; i < argTypes.size(); i++) {
      if (argTypes[i] != itt->second[i]) {
        return false;
      }
    }
    return true;
  } else {
    std::vector<Holmes::HType> sig;
    std::string tableSpec = "(";
    for (size_t i = 0; i < argTypes.size(); i++) {
      tableSpec += "arg" + std::to_string(i) + " " + htype_to_sqltype(argTypes[i]);
      sig.push_back(argTypes[i]);
      if (i == argTypes.size() - 1) {
        tableSpec += ")";
      } else {
        tableSpec += ", ";
      }
    }
    pqxx::work work(conn);
    
    work.exec("CREATE TABLE facts." + name + " " + tableSpec);
    work.commit();
    types[name] = sig;
    registerPrepared(name, sig.size());
    return true;
  }
}

std::string quoteVal(pqxx::work& w, Holmes::Val::Reader v) {
  switch (v.which()) {
    case Holmes::Val::BLOB:
      //You probably don't want to do this... but for completeness sake
      return w.quote_raw(v.getBlob().begin(), v.getBlob().size());
    case Holmes::Val::UINT64:
      //Postgres doesn't support uint64_t
      return w.quote((int64_t)v.getUint64());
    case Holmes::Val::STRING:
      return w.quote(v.getString().cStr());
  }
  throw "Failed to quote value";
}

void buildFromDB(Holmes::HType typ, Holmes::Val::Builder val, pqxx::result::field dbVal) {
  switch (typ) {
    case Holmes::HType::UINT64:
      val.setUint64(dbVal.as<int64_t>());
      break;
    case Holmes::HType::STRING:
      val.setString(dbVal.as<std::string>());
      break;
    case Holmes::HType::BLOB: {
        pqxx::binarystring bs(dbVal);
        auto bb = val.initBlob(bs.size());
        for (size_t k = 0; k < bs.size(); ++k) {
          bb[k] = bs[k];
        }
        break;
      }
  }
}
 
}
