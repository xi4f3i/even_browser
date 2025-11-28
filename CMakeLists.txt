cmake_minimum_required(VERSION 3.10)

project(even_browser)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(glfw3 CONFIG REQUIRED)
find_package(unofficial-skia CONFIG REQUIRED)
find_package(OpenSSL CONFIG REQUIRED)

add_executable(even_browser src/main.cpp)

target_link_libraries(even_browser PRIVATE
    glfw
    unofficial::skia::skia
    OpenSSL::SSL
    OpenSSL::Crypto)
