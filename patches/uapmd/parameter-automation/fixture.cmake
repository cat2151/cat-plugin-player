get_target_property(uh_parameter_includes remidy INCLUDE_DIRECTORIES)
# CPM's download-only variable lives in remidy's directory scope. The recorded
# package location is shared; SDK helper libraries below are test-only targets.
set(vst3sdk_SOURCE_DIR "${CPM_PACKAGE_vst3sdk_SOURCE_DIR}")
function(uh_parameter_fixture_sdk)
    set(SDK_ROOT "${vst3sdk_SOURCE_DIR}")
    list(APPEND CMAKE_MODULE_PATH "${SDK_ROOT}/cmake/modules")
    include(SMTG_VST3_SDK)
    smtg_create_lib_base_target()
    smtg_create_pluginterfaces_target()
    smtg_create_public_sdk_common_target()
    smtg_create_public_sdk_target()
    set_target_properties(base pluginterfaces sdk_common sdk PROPERTIES EXCLUDE_FROM_ALL TRUE)
endfunction()
uh_parameter_fixture_sdk()
add_library(parameter_clap_fixture MODULE EXCLUDE_FROM_ALL "${CMAKE_CURRENT_LIST_DIR}/clap_fixture.cpp")
target_include_directories(parameter_clap_fixture PRIVATE ${uh_parameter_includes})
set_target_properties(parameter_clap_fixture PROPERTIES PREFIX "" SUFFIX ".clap")
add_library(parameter_vst3_fixture MODULE EXCLUDE_FROM_ALL
    "${CMAKE_CURRENT_LIST_DIR}/vst3_fixture.cpp"
    "${vst3sdk_SOURCE_DIR}/public.sdk/source/vst/vstsinglecomponenteffect.cpp"
    "${vst3sdk_SOURCE_DIR}/public.sdk/source/main/dllmain.cpp")
target_include_directories(parameter_vst3_fixture PRIVATE "${vst3sdk_SOURCE_DIR}")
target_link_libraries(parameter_vst3_fixture PRIVATE sdk)
set_target_properties(parameter_vst3_fixture PROPERTIES PREFIX "" SUFFIX ".vst3")
add_executable(parameter_backend_test EXCLUDE_FROM_ALL "${CMAKE_CURRENT_LIST_DIR}/backend_test.cpp")
target_include_directories(parameter_backend_test PRIVATE ${uh_parameter_includes})
target_link_libraries(parameter_backend_test PRIVATE remidy)
