cmake_minimum_required(VERSION 3.21)

project(BrooksCaddyShim DESCRIPTION "A shim of the functions provided by the Brooks library for integration with Caddy")

set(INSTALL_DIR "./install" CACHE PATH "Set the installation directory for the shim library.")

add_library(brooks_lib SHARED shim.c)
add_library(brooks_support support.c)

install(TARGETS brooks_lib DESTINATION "${CMAKE_SOURCE_DIR}/${INSTALL_DIR}")
install(TARGETS brooks_support DESTINATION "${CMAKE_SOURCE_DIR}/${INSTALL_DIR}")
