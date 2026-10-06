#pragma once
#include <filesystem>
#include <cwctype>

namespace uh {
inline bool isVst3Binary(const std::filesystem::path& path) {
    std::error_code error;
    if (!std::filesystem::is_regular_file(path, error)) return false;
    auto extension = path.extension().wstring();
    for (auto& ch : extension) ch = std::towlower(ch);
    return extension == L".vst3";
}

// Do not pass certificates, fonts, helper DLLs or directories to LoadLibrary.
inline std::filesystem::path windowsVst3Binary(const std::filesystem::path& bundle,
                                              const std::filesystem::path& binaryDir) {
    const auto expected = binaryDir / bundle.filename();
    if (isVst3Binary(expected)) return expected;
    std::filesystem::path candidate;
    std::error_code error;
    std::filesystem::directory_iterator iterator(binaryDir, error), end;
    while (!error && iterator != end) {
        if (isVst3Binary(iterator->path())) {
            // Renamed bundles are usable only when their binary is unambiguous.
            if (!candidate.empty()) return {};
            candidate = iterator->path();
        }
        iterator.increment(error);
    }
    return error ? std::filesystem::path{} : candidate;
}
}
