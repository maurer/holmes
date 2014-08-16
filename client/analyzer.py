import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)

nftReq = holmes.newFactType_request()
fs = nftReq.factSig.init('modes', 2)
fs[0].argType = 'string'
fs[0].mode = 'equal'
fs[1].argType = 'addr'
fs[1].mode = 'ignore'
ftid = nftReq.send().wait()

t = ftid.freshFactTypeId

class Analysis(holmes_capnp.Holmes.Analysis.Server):
  def analyze(self, ctx, premises, _context, **kwargs):
    res = []
    for premise in premises:
      der = {'typeId' : t,
             'args'   : [{'stringVal' : premise.args[0].stringVal},
                         {'addrVal'   : 13}]}
      res += [der]
    _context.results.init('derived', len(res))
    _context.results.derived = res

req = holmes.analyzer_request()
req.analysis = Analysis()
premiseBuilder = req.init('premises', 2)
premiseBuilder[0].typeId = 0
premiseBuilder[1].typeId = 0
args = premiseBuilder[1].init('args', 2)
args[0].exactVal = {'stringVal' : "bar"}
args[1].unbound = None
req.send().wait()

print(holmes.derive({'typeId' : t, 'args' : []}).wait())
