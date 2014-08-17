import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)

class Analysis(holmes_capnp.Holmes.Analysis.Server):
  def analyze(self, premises, _context, **kwargs):
    res = []
    for premise in premises:
      der = {'factName' : "derived",
             'args'     : [{'stringVal' : premise.args[0].stringVal},
                           {'addrVal'   : 13}]}
      res += [der]
    _context.results.init('derived', len(res))
    _context.results.derived = res

req = holmes.analyzer_request()
req.analysis = Analysis()
premiseBuilder = req.init('premises', 2)
premiseBuilder[0].factName = "base"
premiseBuilder[1].factName = "base"
args = premiseBuilder[1].init('args', 2)
args[0].exactVal = {'stringVal' : "bar"}
args[1].unbound = None
req.send().wait()

print(holmes.derive({'factName' : "derived", 'args' : []}).wait())
