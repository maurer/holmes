macro(CAPNPROTO_ADD_CXXFLAGS)
    include(CheckCXXCompilerFlag)
    CHECK_CXX_COMPILER_FLAG("-std=c++11" COMPILER_SUPPORTS_CXX11)
    CHECK_CXX_COMPILER_FLAG("-std=c++0x" COMPILER_SUPPORTS_CXX0X)
    if(COMPILER_SUPPORTS_CXX11)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++11")
    elseif(COMPILER_SUPPORTS_CXX0X)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++0x")
    else()
        message(STATUS "The compiler ${CMAKE_CXX_COMPILER} has no C++11 support. Please use a different C++ compiler.")
    endif()
endmacro()

function(CAPNPROTO_GENERATE_CPP SRCS HDRS)
  if(NOT ARGN)
    message(SEND_ERROR "Error: CAPNPROTO_GENERATE_CPP() called without any capnp files")
    return()
  endif()

  if(CAPNPROTO_GENERATE_CPP_APPEND_PATH)
    # Create an include path for each file specified
    foreach(FIL ${ARGN})
      get_filename_component(ABS_FIL ${FIL} ABSOLUTE)
      get_filename_component(ABS_PATH ${ABS_FIL} PATH)
      list(FIND _capnproto_include_path ${ABS_PATH} _contains_already)
      if(${_contains_already} EQUAL -1)
          list(APPEND _capnproto_include_path -I ${ABS_PATH})
      endif()
    endforeach()
  else()
    set(_capnproto_include_path -I ${CMAKE_CURRENT_SOURCE_DIR})
  endif()

  if(DEFINED CAPNPROTO_IMPORT_DIRS)
    foreach(DIR ${CAPNPROTO_IMPORT_DIRS})
      get_filename_component(ABS_PATH ${DIR} ABSOLUTE)
      list(FIND _capnproto_include_path ${ABS_PATH} _contains_already)
      if(${_contains_already} EQUAL -1)
          list(APPEND _capnproto_include_path -I ${ABS_PATH})
      endif()
    endforeach()
  endif()

  set(${SRCS})
  set(${HDRS})
  foreach(FIL ${ARGN})
    get_filename_component(ABS_FIL ${FIL} ABSOLUTE)
    get_filename_component(FIL_WE ${FIL} NAME_WE)
    get_filename_component(FIL_DIR ${ABS_FIL} DIRECTORY)

    list(APPEND ${SRCS} "${CMAKE_CURRENT_BINARY_DIR}/${FIL_WE}.capnp.c++")
    list(APPEND ${HDRS} "${CMAKE_CURRENT_BINARY_DIR}/${FIL_WE}.capnp.h")

    add_custom_command(
      OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/${FIL_WE}.capnp.c++"
             "${CMAKE_CURRENT_BINARY_DIR}/${FIL_WE}.capnp.h"
      COMMAND  ${CAPNPROTO_CAPNPC_EXECUTABLE}
      ARGS ${_capnproto_include_path} ${ABS_FIL} --src-prefix=${FIL_DIR} -o c++:${CMAKE_CURRENT_BINARY_DIR}
      DEPENDS ${ABS_FIL}
      COMMENT "Running C++ Cap'n Proto compiler on ${FIL}"
      VERBATIM )
  endforeach()

  set_source_files_properties(${${SRCS}} ${${HDRS}} PROPERTIES GENERATED TRUE)
  set(${SRCS} ${${SRCS}} PARENT_SCOPE)
  set(${HDRS} ${${HDRS}} PARENT_SCOPE)
endfunction()

find_library(CAPNPROTO_LIBRARY NAMES capnp PATHS)
mark_as_advanced(CAPNPROTO_LIBRARY)
find_library(CAPNPROTO_LIBRARY_DEBUG NAMES capnp PATHS)
mark_as_advanced(CAPNPROTO_LIBRARY_DEBUG)
find_library(CAPNPROTO_RPC_LIBRARY NAMES capnp-rpc PATHS)
mark_as_advanced(CAPNPROTO_RPC_LIBRARY)
find_library(CAPNPROTO_RPC_LIBRARY_DEBUG NAMES capnp-rpc PATHS)
mark_as_advanced(CAPNPROTO_RPC_LIBRARY_DEBUG)
set(CAPNPROTO_BASE_LIBRARIES
        optimized ${CAPNPROTO_LIBRARY}
        debug     ${CAPNPROTO_LIBRARY_DEBUG}
        )
set(CAPNPROTO_RPC_LIBRARIES
        optimized ${CAPNPROTO_RPC_LIBRARY}
        debug     ${CAPNPROTO_RPC_LIBRARY_DEBUG}
        )

if(NOT DEFINED CAPNPROTO_GENERATE_CPP_APPEND_PATH)
  set(CAPNPROTO_GENERATE_CPP_APPEND_PATH TRUE)
endif()

# Find the include directory
find_path(CAPNPROTO_INCLUDE_DIR
    capnp/ez-rpc.h
    PATHS ${CAPNPROTO_SRC_ROOT_FOLDER}/src
)
mark_as_advanced(CAPNPROTO_INCLUDE_DIR)

find_program(CAPNPROTO_CAPNPC_EXECUTABLE
    NAMES capnpc
    DOC "Cap'n Proto Compiler"
    PATHS
)
mark_as_advanced(CAPNPROTO_CAPNPC_EXECUTABLE)

find_package(KJ REQUIRED)

include(FindPackageHandleStandardArgs)
FIND_PACKAGE_HANDLE_STANDARD_ARGS(CAPNPROTO DEFAULT_MSG CAPNPROTO_LIBRARY CAPNPROTO_INCLUDE_DIR)
set(CAPNPROTO_INCLUDE_DIRS ${CAPNPROTO_INCLUDE_DIR})
set(CAPNPROTO_LIBRARIES ${CAPNPROTO_BASE_LIBRARIES} ${CAPNPROTO_RPC_LIBRARIES} ${KJ_LIBRARIES})
