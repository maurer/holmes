import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)

class Analysis(holmes_capnp.Holmes.Analysis.Server):
  def analyze(self, ctx, premises, **kwargs):
    print(str(ctx))
    print(str(premises))
    return [{'typeId' : 0, 'args' : [{'stringVal' : 'bazarus'}, {'addrVal' : 32}]}]

req = holmes.analyzer_request()
req.analysis = Analysis()
premiseBuilder = req.init('premises', 2)
premiseBuilder[0].typeId = 0
premiseBuilder[1].typeId = 0
args = premiseBuilder[1].init('args', 2)
args[0].exactVal = {'stringVal' : "bar"}
args[1].unbound = None
req.send().wait()

print(holmes.derive({'typeId' : 0, 'args' : []}).wait())
