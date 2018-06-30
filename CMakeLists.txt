cmake_minimum_required(VERSION 3.8)
project(rust_oids)

set(CMAKE_CXX_STANDARD 11)

set(SOURCE_FILES
        Cargo.toml)

add_executable(rust_oids ${SOURCE_FILES})