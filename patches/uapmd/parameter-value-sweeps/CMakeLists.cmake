find_package(Python3 REQUIRED COMPONENTS Interpreter)
set(uh_values_patch "${CMAKE_CURRENT_LIST_DIR}")
set(uh_values_input "${CMAKE_CURRENT_BINARY_DIR}/generated/parameter-automation")
set(uh_values_output "${CMAKE_CURRENT_BINARY_DIR}/generated/parameter-value-sweeps")
set(uh_values_host "${uh_values_input}/clap/HostClasses.cpp")
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS
    "${uh_values_patch}/apply.py" "${uh_values_host}")
execute_process(COMMAND "${Python3_EXECUTABLE}" -B "${uh_values_patch}/apply.py"
    "${UAPMD_DIR}/source/remidy/src"
    "${CMAKE_CURRENT_BINARY_DIR}/generated/clap-note-dialect"
    "${CMAKE_CURRENT_BINARY_DIR}/generated/clap-state-load-error"
    "${uh_values_input}" "${uh_values_output}"
    RESULT_VARIABLE uh_values_result ERROR_VARIABLE uh_values_error OUTPUT_QUIET)
if(NOT uh_values_result EQUAL 0)
    message(FATAL_ERROR "${uh_values_error}")
endif()
get_target_property(uh_values_sources remidy SOURCES)
execute_process(COMMAND git -C "${UAPMD_DIR}" rev-parse HEAD
    OUTPUT_VARIABLE uh_values_revision OUTPUT_STRIP_TRAILING_WHITESPACE ERROR_QUIET)
if(NOT uh_values_revision)
    set(uh_values_revision "unknown")
endif()
set(uh_values_count 0)
foreach(uh_values_source IN LISTS uh_values_sources)
    get_filename_component(uh_values_basename "${uh_values_source}" NAME)
    if(uh_values_basename STREQUAL "HostClasses.cpp" AND uh_values_source MATCHES "(^|/)clap/")
        if(NOT uh_values_source STREQUAL uh_values_host)
            message(FATAL_ERROR "[parameter-value-sweeps] target: ${uh_values_source}; unexpected composed source, expected ${uh_values_host}; UAPMD revision: ${uh_values_revision}; instructions: ${uh_values_patch}/README.md")
        endif()
        list(REMOVE_ITEM uh_values_sources "${uh_values_source}")
        math(EXPR uh_values_count "${uh_values_count} + 1")
    endif()
endforeach()
if(NOT uh_values_count EQUAL 1)
    message(FATAL_ERROR "[parameter-value-sweeps] target: ${uh_values_host}; expected one remidy source, found ${uh_values_count}; UAPMD revision: ${uh_values_revision}; instructions: ${uh_values_patch}/README.md")
endif()
list(APPEND uh_values_sources "${uh_values_output}/HostClasses.cpp")
set_property(TARGET remidy PROPERTY SOURCES "${uh_values_sources}")
