cmake_minimum_required(VERSION 3.28)
project(uapmd_shim LANGUAGES C CXX)

# Where the uapmd checkout is. Pin it to a commit you have built successfully:
# its API is still changing.
set(UAPMD_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../uapmd" CACHE PATH "Path to the uapmd repository root")
if(NOT EXISTS "${UAPMD_DIR}/source/CMakeLists.txt")
    message(FATAL_ERROR "uapmd not found at '${UAPMD_DIR}'. Pass -DUAPMD_DIR=<path to the uapmd checkout>.")
endif()

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(MSVC)
    # /utf-8: uapmd's sources are UTF-8 without BOM; without this a Japanese (CP932)
    #         system miscompiles them. Set here so that it reaches uapmd as well.
    # /EHsc:  stated explicitly so that it does not depend on who drives CMake.
    add_compile_options(/utf-8 /EHsc)
    add_compile_definitions(NOMINMAX)
endif()

# Same choices as the Windows build line in uapmd's README.
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(REMIDY_BUILD_CONFIG Release CACHE STRING "" FORCE)
set(UAPMD_BUILD_TESTS OFF CACHE BOOL "" FORCE)

# The whole of uapmd gets configured, but EXCLUDE_FROM_ALL means that only what
# uapmd_shim links against gets compiled.
add_subdirectory("${UAPMD_DIR}" uapmd EXCLUDE_FROM_ALL)
include("${CMAKE_CURRENT_SOURCE_DIR}/../patches/uapmd/windows-vst3-loader/CMakeLists.cmake")

add_library(uapmd_shim SHARED uapmd_shim.cpp uapmd_shim.h plugin_catalog.cpp plugin_catalog.h)
if(WIN32)
    target_link_libraries(uapmd_shim PRIVATE gdi32)
endif()
get_target_property(REMIDY_METADATA_INCLUDES remidy INCLUDE_DIRECTORIES)
target_include_directories(uapmd_shim PRIVATE ${REMIDY_METADATA_INCLUDES})
target_compile_definitions(uapmd_shim PRIVATE UAPMD_SHIM_BUILD)
# uapmd declares these include directories PRIVATE, so they are repeated here.
target_include_directories(uapmd_shim PRIVATE
        "${UAPMD_DIR}/source/remidy/include"
        "${UAPMD_DIR}/source/uapmd-plugin-hosting/include"
        "${UAPMD_DIR}/source/remidy-gui/include"
)
target_link_libraries(uapmd_shim PRIVATE
        uapmd-plugin-hosting
        remidy
        remidy-gui
)

# One fixed output directory for every configuration, so that build.rs can find the files.
set(UH_OUT_DIR "${CMAKE_BINARY_DIR}/out")
set_target_properties(uapmd_shim PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY "$<1:${UH_OUT_DIR}>"
        LIBRARY_OUTPUT_DIRECTORY "$<1:${UH_OUT_DIR}>"
        ARCHIVE_OUTPUT_DIRECTORY "$<1:${UH_OUT_DIR}>"
)
