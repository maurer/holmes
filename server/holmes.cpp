#include <capnp/ez-rpc.h>

#include <iostream>
using namespace std;

int main (int argc, char** argv) {
  capnp::EzRpcServer server("*");
  auto& waitScope = server.getWaitScope();
  auto port = server.getPort().wait(waitScope);
  cout << port;
}
