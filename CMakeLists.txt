cmake_minimum_required(VERSION 2.8)
project(Holmes)
set(CMAKE_MODULE_PATH
    ${CMAKE_MODULE_PATH}
    "${CMAKE_SOURCE_DIR}/cmake/Modules/"
   )
add_subdirectory("server")
