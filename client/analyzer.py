import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)

class Analysis(holmes_capnp.Holmes.Analysis.Server):
  def analyze(self, ctx, **kwargs):
    #print("ctx="+str(ctx))
    return [{'typeId' : 0, 'args' : [{'stringVal' : 'bazarus'}, {'addrVal' : 32}]}]

req = holmes.analyzer_request()
req.analysis = Analysis()
req.send().wait()

print(holmes.derive({'typeId' : 0, 'args' : []}).wait())
