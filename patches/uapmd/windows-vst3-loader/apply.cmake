# Reusable in script mode for regression tests; no dependency checkout is edited.
function(uh_vst3_patch_error source condition)
    if(EXISTS "${source}")
        file(SHA256 "${source}" source_hash)
    else()
        set(source_hash "unavailable (source missing)")
    endif()
    get_filename_component(source_dir "${source}" DIRECTORY)
    execute_process(COMMAND git -C "${source_dir}" rev-parse HEAD
        OUTPUT_VARIABLE revision OUTPUT_STRIP_TRAILING_WHITESPACE ERROR_QUIET)
    if(NOT revision)
        set(revision "unknown (Git metadata unavailable)")
    endif()
    message(FATAL_ERROR
        "[cat-plugin-player: windows-vst3-loader] Patch was NOT applied.\n"
        "Source: ${source}\n"
        "UAPMD revision: ${revision}\n"
        "Source SHA256: ${source_hash}\n"
        "Failed condition: ${condition}\n"
        "The upstream loader may have changed or already been fixed.\n"
        "Compare the source with the documented baseline; update the patch or remove it only after verifying the upstream fix. Do not bypass the patch.\n"
        "Instructions: ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/README.md")
endfunction()

function(uh_vst3_replace_once source content_var expected replacement label)
    set(content "${${content_var}}")
    string(REPLACE "${expected}" "" stripped "${content}")
    string(LENGTH "${content}" before_length)
    string(LENGTH "${stripped}" after_length)
    string(LENGTH "${expected}" expected_length)
    math(EXPR matches "(${before_length} - ${after_length}) / ${expected_length}")
    if(NOT matches EQUAL 1)
        uh_vst3_patch_error("${source}" "${label}: expected exactly 1 occurrence, found ${matches}")
    endif()
    string(REPLACE "${expected}" "${replacement}" content "${content}")
    set(${content_var} "${content}" PARENT_SCOPE)
endfunction()

function(uh_patch_windows_vst3_loader source output)
    if(NOT EXISTS "${source}")
        uh_vst3_patch_error("${source}" "ClassModuleInfo.cpp must exist")
    endif()
    get_filename_component(source_dir "${source}" DIRECTORY)
    file(READ "${source}" content)
    string(REPLACE "\r\n" "\n" content "${content}")
    uh_vst3_replace_once("${source}" content
        "#include \"ClassModuleInfo.hpp\""
        "#include \"${source_dir}/ClassModuleInfo.hpp\"\n#include \"${CMAKE_CURRENT_FUNCTION_LIST_DIR}/windows_vst3_binary.h\""
        "module info header include")
    uh_vst3_replace_once("${source}" content
        "#include \"../utils.hpp\"" "#include \"${source_dir}/../utils.hpp\""
        "loader utilities header include")
    uh_vst3_replace_once("${source}" content
        "        if (!is_directory(pluginPath)) // self-contained plugin DLL\n            return pluginPath;"
        "        if (!is_directory(pluginPath)) // self-contained plugin DLL\n            return uh::isVst3Binary(pluginPath) ? pluginPath : std::filesystem::path{};"
        "self-contained VST3 binary validation")
    # Upstream mutates pluginPath to form binDir. Recover the original bundle
    # from binDir instead of accidentally using the ABI directory's filename.
    uh_vst3_replace_once("${source}" content
        "        for (auto& entry : std::filesystem::directory_iterator(binDir))\n            return entry.path();\n        return {};"
        "        return uh::windowsVst3Binary(binDir.parent_path().parent_path(), binDir);"
        "unsafe first-entry binary selection")
    file(CONFIGURE OUTPUT "${output}" CONTENT "${content}" @ONLY)
    message(STATUS "[cat-plugin-player: windows-vst3-loader] Applied to build-local copy: ${output}; dependency checkout unchanged")
endfunction()
