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
          for (auto&& i : context.ctx) {
            kj::Own<MMB> mb = kj::heap<MMB>();
            mb->setRoot(i.second);
            ctx[i.first] = mb->getRoot<Holmes::Val>();
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
    struct FactAssignment {
      FactAssignment(){}
      FactAssignment(Context ctx, std::vector<Holmes::Fact::Reader> facts)
        : context(kj::mv(ctx))
        , facts(facts) {}
      Context context;
      std::vector<Holmes::Fact::Reader> facts;
      inline void combine(FactAssignment &x){
        context.insert(x.context.begin(), x.context.end());
        facts.insert(facts.begin(), x.facts.begin(), x.facts.end());
      }
    };
    class FactResults {
      public:
        FactResults() {}
        std::vector<FactAssignment> results;
        std::set<capnp::MallocMessageBuilder*> mbs;
        ~FactResults() {
          for (auto mb : mbs) {
            delete mb;
          }
        }
        FactResults(FactResults&&) = default;
        FactResults& operator=(FactResults&&) = default;
      private:
        KJ_DISALLOW_COPY(FactResults);
    };
    virtual ~DAL(){}
    virtual bool setFact(Holmes::Fact::Reader) = 0;
    virtual size_t setFacts(capnp::List<Holmes::Fact>::Reader facts) {
      size_t f = 0;
      for (auto fact : facts) {
        if (setFact(fact)) {
          f++;
        }
      }
      return f;
    }
    virtual bool addType(std::string, capnp::List<Holmes::HType>::Reader) = 0;
    virtual FactResults getFacts(
      Holmes::FactTemplate::Reader,
      Context ctx) = 0;
    virtual FactResults getFacts(capnp::List<Holmes::FactTemplate>::Reader premises) {
      std::vector<FactAssignment> fas;
      fas.push_back(FactAssignment());
      std::vector<FactResults> frs;
      for (auto premise : premises) {
        std::vector<FactAssignment> newFas;
        for (auto&& fa : fas) {
          auto newFr = getFacts(premise, fa.context);
          DLOG(INFO) << "Search step got " << newFr.results.size() << " results.";
          for (auto&& newFa : newFr.results) {
            newFa.combine(fa);
            newFas.push_back(newFa);
          }
          frs.push_back(kj::mv(newFr));
        }
        fas = kj::mv(newFas);
      }
      FactResults x;
      for (auto&& fr : frs) {
        x.mbs.insert(fr.mbs.begin(), fr.mbs.end());
        fr.mbs.clear();
      }
      DLOG(INFO) << "Returning " << fas.size() << " results.";
      x.results = kj::mv(fas);
      return x;
    }
};

}

#endif
