find_package(Capnproto REQUIRED)
find_package(PQXX REQUIRED)
find_package(PostgreSQL REQUIRED)
find_package(Glog REQUIRED)
include_directories(${CAPNPROTO_INCLUDE_DIRS} ${GFLAGS_INCLUDE_DIR}
        ${GLOG_INCLUDE_DIR} ${PQXX_INCLUDE_DIR})
CAPNPROTO_GENERATE_CPP(CAPN_SRCS CAPN_HDRS holmes.capnp)
include_directories(${CMAKE_CURRENT_BINARY_DIR})
add_executable(holmes holmes.cpp analyzer.cpp pgDal.cpp
        fact_util.cpp ${CAPN_SRCS} ${CAPN_HDRS})
target_link_libraries(holmes ${CAPNPROTO_LIBRARIES}
                      ${GLOG_LIBRARIES}
                      ${PostgreSQL_LIBRARIES} ${PQXX_LIBRARIES} pthread)
CAPNPROTO_ADD_CXXFLAGS()
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Werror")
