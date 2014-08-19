import sys
import capnp
import holmes_capnp
import inspect
client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)
for pred in sys.argv[2:]:
  print(holmes.derive({'factName' : pred}).wait())
