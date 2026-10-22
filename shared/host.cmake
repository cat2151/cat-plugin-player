set(UH_NATIVE_DIR "${CMAKE_CURRENT_LIST_DIR}/../shim")
set(UH_PLAYER_ROOT "${CMAKE_CURRENT_LIST_DIR}/..")
# Where the uapmd checkout is. Pin it to a commit you have built successfully:
# its API is still changing.
set(UAPMD_DIR "${UH_PLAYER_ROOT}/../uapmd" CACHE PATH "Path to the uapmd repository root")
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
include("${UH_PLAYER_ROOT}/patches/uapmd/stb-revision-pin/CMakeLists.cmake")
add_subdirectory("${UAPMD_DIR}" uapmd EXCLUDE_FROM_ALL)
uh_stb_verify()
include("${UH_PLAYER_ROOT}/patches/uapmd/clap-note-dialect/CMakeLists.cmake")
include("${UH_PLAYER_ROOT}/patches/uapmd/clap-state-load-error/CMakeLists.cmake")
include("${UH_PLAYER_ROOT}/patches/uapmd/scan-tool-data-dir-race/CMakeLists.cmake")
include("${UH_PLAYER_ROOT}/patches/uapmd/parameter-automation/CMakeLists.cmake")
include("${UH_PLAYER_ROOT}/patches/uapmd/parameter-automation/fixture.cmake")

add_library(uapmd_shim SHARED
    "${UH_NATIVE_DIR}/uapmd_shim.cpp" "${UH_NATIVE_DIR}/uapmd_shim.h"
    "${UH_NATIVE_DIR}/plugin_catalog.cpp" "${UH_NATIVE_DIR}/plugin_catalog.h"
    "${UH_NATIVE_DIR}/shutdown_overlay.cpp")
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
        "${UAPMD_DIR}/source/uapmd-graph/include"
)
target_link_libraries(uapmd_shim PRIVATE
        uapmd-plugin-hosting
        uapmd-graph
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

# build.rs passes MultiThreaded when Rust links the CRT statically. uapmd forces the DLL
# runtime both in the cache and on some targets, so the runtime is set on every target
# here, after uapmd and its dependencies have been added. Empty keeps uapmd's choice.
if(UH_MSVC_RUNTIME_LIBRARY)
    function(uh_set_msvc_runtime dir)
        get_property(targets DIRECTORY "${dir}" PROPERTY BUILDSYSTEM_TARGETS)
        foreach(target IN LISTS targets)
            set_property(TARGET ${target} PROPERTY MSVC_RUNTIME_LIBRARY "${UH_MSVC_RUNTIME_LIBRARY}")
        endforeach()
        get_property(subdirs DIRECTORY "${dir}" PROPERTY SUBDIRECTORIES)
        foreach(subdir IN LISTS subdirs)
            uh_set_msvc_runtime("${subdir}")
        endforeach()
    endfunction()
    uh_set_msvc_runtime("${CMAKE_CURRENT_SOURCE_DIR}")
endif()

# Minimal SMF bridge shares this same UAPMD configure and dependency tree.
add_library(smf_shim STATIC "${UH_PLAYER_ROOT}/shared/smf_shim.cpp")
target_link_libraries(smf_shim PRIVATE umppi_static)
set_target_properties(smf_shim PROPERTIES ARCHIVE_OUTPUT_DIRECTORY "$<1:${CMAKE_BINARY_DIR}/out>")
if(UH_MSVC_RUNTIME_LIBRARY)
    set_target_properties(smf_shim PROPERTIES MSVC_RUNTIME_LIBRARY "${UH_MSVC_RUNTIME_LIBRARY}")
endif()
