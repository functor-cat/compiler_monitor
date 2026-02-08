cmake_minimum_required(VERSION 3.15)
project(CompilerMonitorTest)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Add executable
add_executable(${PROJECT_NAME}
    src/main.cpp
    src/mathops.cpp
    src/mathops.h
    src/utils.cpp
    src/utils.h
)

# Add library to test multiple compilation units
add_library(calculator STATIC
    src/calculator.cpp
    src/calculator.h
)

target_link_libraries(${PROJECT_NAME} PRIVATE calculator)

# Add include directories
target_include_directories(${PROJECT_NAME} PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)
