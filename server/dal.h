#ifndef HOLMES_SERVER_DAL_H_
#define HOLMES_SERVER_DAL_H_

#include <vector>
#include <set>
#include <map>

#include <kj/common.h>
#include <capnp/message.h>
#include <glog/logging.h>

#include "holmes.capnp.h"

namespace holmes {

class DAL {
  public:
    class Context {
      typedef capnp::MallocMessageBuilder MMB;
      typedef std::map<std::string, Holmes::Val::Reader> Ctx;
      private:
        std::vector<kj::Own<MMB>> mbs;
        Ctx ctx;
      public:
        Ctx::const_iterator begin() const {
          return ctx.begin();
        }
        Ctx::const_iterator end() const {
          return ctx.end();
        }
        Ctx::const_iterator find(std::string k) const {
          return ctx.find(k);
        }
        size_t size() const {
          return ctx.size();
        }
        Holmes::Val::Builder init(const Ctx::key_type& k) {
          kj::Own<MMB> mb = kj::heap<MMB>();
          auto vb = mb->initRoot<Holmes::Val>();
          ctx[k] = mb->getRoot<Holmes::Val>();
          mbs.push_back(kj::mv(mb));
          return vb;
        }
        template <class InputIterator>
        void insert (InputIterator first, InputIterator last) {
          for (auto i = first; i != last; ++i) {
            kj::Own<MMB> mb = kj::heap<MMB>();
            mb->setRoot(i->second);
            ctx[i->first] = mb->getRoot<Holmes::Val>();
            mbs.push_back(kj::mv(mb));
          }
        }
        void insert(std::pair<std::string, Holmes::Val::Reader> i) {
          kj::Own<MMB> mb = kj::heap<MMB>();;
          mb->setRoot(i.second);
          ctx[i.first] = mb->getRoot<Holmes::Val>();
          mbs.push_back(kj::mv(mb));
        }
        Ctx::mapped_type& operator[] (const Ctx::key_type& k) {
          return ctx[k];
        }
        Context(const Context& context) {
          for (auto i = context.ctx.begin(); i != context.ctx.end(); ++i) {
            kj::Own<MMB> mb = kj::heap<MMB>();
            mb->setRoot(i->second);
            ctx[i->first] = mb->getRoot<Holmes::Val>();
            mbs.push_back(kj::mv(mb));
          }
        }
        Context(Context&&) = default;
        Context& operator=(Context&&) = default;
        Context& operator=(const Context& context) {
          for (auto&& i : context.ctx) {
            kj::Own<MMB> mb = kj::heap<MMB>();
            mb->setRoot(i.second);
            ctx[i.first] = mb->getRoot<Holmes::Val>();
            mbs.push_back(kj::mv(mb));
          }
          return *this;
        }
        Context() = default;
    };
    virtual ~DAL(){}
    virtual size_t setFacts(capnp::List<Holmes::Fact>::Reader facts) = 0;
    virtual bool addType(std::string, capnp::List<Holmes::HType>::Reader) = 0;
    virtual std::vector<Context> getFacts(capnp::List<Holmes::FactTemplate>::Reader premises) = 0;
};

}

#endif
