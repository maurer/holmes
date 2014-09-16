#ifndef __HOLMES_GLOG__
#define __HOLMES_GLOG__

#if USE_GLOG
#include <glog/logging.h>
#else
#include <iostream>
#define DLOG(x) std::cerr
#define LOG(x) std::cerr
#endif

#endif
