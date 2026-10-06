#include "windows_vst3_binary.h"
#include <chrono>
#include <fstream>
#include <iostream>
#include <stdexcept>

namespace remidy_vst3 {
std::filesystem::path getPluginCodeFile(std::filesystem::path& pluginPath);
}

int main(int argc, char** argv) {
    namespace fs = std::filesystem;
    const auto root = fs::temp_directory_path() / ("cat-vst3-test-" +
        std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    const auto bundle = root / "Reason Rack Plugin.vst3";
    const auto binaries = bundle / "Contents" / "x86_64-win";
    auto require = [](bool condition) {
        if (!condition) throw std::runtime_error("incorrect VST3 binary selection");
    };
    try {
        fs::create_directories(binaries / "Metadata");
        fs::create_directories(binaries / "fake.vst3");
        for (const auto* resource : {"cacert.pem", "credits.txt", "Inter-Regular.otf", "helper.dll"})
            std::ofstream(binaries / resource) << "resource";
        require(uh::windowsVst3Binary(bundle, binaries).empty());
        require(uh::windowsVst3Binary(bundle, binaries / "missing").empty());
        const auto binary = binaries / bundle.filename();
        std::ofstream(binary) << "binary";
        require(uh::windowsVst3Binary(bundle, binaries) == binary);
        require(uh::isVst3Binary(binary));
        auto upstreamPath = bundle;
        require(remidy_vst3::getPluginCodeFile(upstreamPath) == binary);
        upstreamPath = binary;
        require(remidy_vst3::getPluginCodeFile(upstreamPath) == binary);
        upstreamPath = binaries / "cacert.pem";
        require(remidy_vst3::getPluginCodeFile(upstreamPath).empty());
        std::ofstream(binaries / "other.vst3") << "binary";
        require(uh::windowsVst3Binary(bundle, binaries) == binary);
        fs::remove(binary);
        require(uh::windowsVst3Binary(bundle, binaries) == binaries / "other.vst3");
        std::ofstream(binaries / "second.VST3") << "binary";
        require(uh::windowsVst3Binary(bundle, binaries).empty());
        fs::remove(binaries / "other.vst3");
        require(uh::windowsVst3Binary(bundle, binaries) == binaries / "second.VST3");
        require(!uh::isVst3Binary(binaries / "cacert.pem"));
        require(!uh::isVst3Binary(binaries / "fake.vst3"));
        if (argc == 2) {
            const fs::path installed(argv[1]);
            require(uh::windowsVst3Binary(installed, installed / "Contents" / "x86_64-win") ==
                installed / "Contents" / "x86_64-win" / installed.filename());
            upstreamPath = installed;
            require(remidy_vst3::getPluginCodeFile(upstreamPath) ==
                installed / "Contents" / "x86_64-win" / installed.filename());
            std::cout << "PASS: installed bundle selects its VST3 binary\n";
        }
        fs::remove_all(root);
        std::cout << "PASS: resources, directories, missing/ambiguous binaries and renamed bundles\n";
    } catch (const std::exception& error) {
        fs::remove_all(root);
        std::cerr << error.what() << '\n';
        return 1;
    }
}
