cmake_minimum_required(VERSION 3.20)

project(melpp)

add_executable(melpp mel.cpp)
set_property(TARGET melpp PROPERTY CXX_STANDARD 20)
