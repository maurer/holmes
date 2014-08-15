import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)
nftReq = holmes.newFactType_request()
fs = nftReq.factSig.init('modes', 2);
fs[0].argType = 'string';
fs[0].mode = 'equal'
fs[1].argType = 'addr'
fs[1].mode = 'ignore'
ftid = nftReq.send().wait()
print("Created fact: " + str(ftid))
holmes.set({'typeId' : ftid.freshFactTypeId,
            'args' : [{'stringVal' : "foo"}, {'addrVal' : 7}]}).wait()
print("Fact submitted.")
print(holmes.derive({'typeId' : ftid,
                     'args' : [{'unbound' : None}, {'unbound' : None}]}).wait())
