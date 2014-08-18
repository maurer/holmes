import sys
import capnp
import holmes_capnp
import inspect

fileName = sys.argv[2]

with open(fileName, mode='rb') as file:
    fileContent = file.read()

client = capnp.TwoPartyClient("localhost:" + sys.argv[1])
holmes = client.ez_restore('holmes').cast_as(holmes_capnp.Holmes)
holmes.set({'factName' : "file",
            'args'     : [{'stringVal' : fileName},
                          {'blobVal'   : fileContent}]}).wait()
