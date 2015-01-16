#include <iostream>

#include <capnp/ez-rpc.h>

#include "glog.h"
#include "holmes.h"
#include "pgDal.h"

using kj::Own;
using kj::heap;
using kj::mv;

int main(int argc, char* argv[]) {
  #ifdef USE_GLOG
  google::InitGoogleLogging(argv[0]);
  #endif
  Own<holmes::DAL> base = heap<holmes::PgDAL>();
  capnp::EzRpcServer server(heap<holmes::HolmesImpl>(mv(base)), "*");

  auto &waitScope = server.getWaitScope();
  uint port = server.getPort().wait(waitScope);
  LOG(INFO) << "Running on port: " << port;
  std::cout << port << std::endl;
  kj::NEVER_DONE.wait(waitScope);
}
