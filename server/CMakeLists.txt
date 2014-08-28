find_package(Capnproto REQUIRED)
find_package(Gflags REQUIRED)
find_package(Glog REQUIRED)
include_directories(${CAPNPROTO_INCLUDE_DIRS} ${GFLAGS_INCLUDE_DIR} ${GLOG_INCLUDE_DIR})
CAPNPROTO_GENERATE_CPP(CAPN_SRCS CAPN_HDRS holmes.capnp)
include_directories(${CMAKE_CURRENT_BINARY_DIR})
add_executable(holmes holmes.cpp memDal.cpp analyzer.cpp
                      ${CAPN_SRCS} ${CAPN_HDRS})
target_link_libraries(holmes ${CAPNPROTO_LIBRARIES}
                      ${GFLAGS_LIBRARIES} ${GLOG_LIBRARIES})
CAPNPROTO_ADD_CXXFLAGS()
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Werror")
