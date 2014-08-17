import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)
holmes.set({'factName' : "base",
            'args'     : [{'stringVal' : "foo"}, {'addrVal' : 7}]}).wait()
holmes.set({'factName' : "base", 
            'args'     : [{'stringVal' : "bar"}, {'addrVal' : 8}]}).wait()
derReq = holmes.derive_request()
derReq.target.factName = "base"
args = derReq.target.init('args', 2)
args[0].unbound = None
args[1].exactVal.addrVal = 8
res = derReq.send().wait()
print(res)
