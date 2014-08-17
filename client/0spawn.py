import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)
nftReq = holmes.newFactType_request()
holmes.set({'typeId' : 0, 
            'args' : [{'stringVal' : "bar"}, {'addrVal' : 7}]}).wait()
holmes.set({'typeId' : 0, 
            'args' : [{'stringVal' : "bar"}, {'addrVal' : 8}]}).wait()
