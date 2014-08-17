import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)
derReq = holmes.derive_request()
derReq.target.factName = sys.argv[2]
res = derReq.send().wait()
print(res)
