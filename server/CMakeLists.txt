find_package(Capnproto REQUIRED)
find_package(PQXX REQUIRED)
find_package(PostgreSQL REQUIRED)
if(USE_GLOG)
  find_package(Glog REQUIRED)
endif()
include_directories(${CAPNPROTO_INCLUDE_DIRS}
        ${GLOG_INCLUDE_DIR} ${PQXX_INCLUDE_DIR})
CAPNPROTO_GENERATE_CPP(CAPN_SRCS CAPN_HDRS holmes.capnp)
include_directories(${CMAKE_CURRENT_BINARY_DIR})
add_executable(holmes holmes.cpp analyzer.cpp pgDal.cpp
        fact_util.cpp ${CAPN_SRCS} ${CAPN_HDRS})
target_link_libraries(holmes ${CAPNPROTO_LIBRARIES}
                      ${GLOG_LIBRARIES}
                      ${PostgreSQL_LIBRARIES} ${PQXX_LIBRARIES} pthread)
CAPNPROTO_ADD_CXXFLAGS()
set(CMAKE_CXX_FLAGS "-Wall -Werror ${CMAKE_CXX_FLAGS}")
if(USE_GLOG)
        set(CMAKE_CXX_FLAGS "-DUSE_GLOG ${CMAKE_CXXFLAGS}")
endif()
install(TARGETS holmes
        RUNTIME DESTINATION bin)
