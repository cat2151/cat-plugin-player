#include "plugin_catalog.h"
#include <remidy/remidy.hpp>
#include <algorithm>
#include <cstring>
#include <set>
#if _WIN32
#include <Windows.h>
#include "windows_vst3_binary.h"
#include <clap/clap.h>
#include <pluginterfaces/base/ipluginbase.h>
#include <pluginterfaces/vst/ivstaudioprocessor.h>
#endif

using namespace uapmd_plugin_hosting;

void completeClapCatalog(AudioPluginHostingAPI& api, std::vector<AudioPluginCatalogEntry>& entries) {
    auto tool = PluginScanTool::create();
    bool changed = false;
    for (auto* format : api.pluginFormats()) {
        if (format->name() != "CLAP")
            continue;
        auto* scanner = dynamic_cast<AudioPluginFileOrUrlScanning*>(format->scanning());
        if (!scanner)
            continue;
        // Populates the upstream scanner's pending list, including Windows DLL files.
        scanner->getAllFastScannablePlugins();
        for (const auto& bundle : scanner->enumerateCandidateBundles(false)) {
            if (tool->isBundleBlocklisted("CLAP", bundle))
                continue;
            if (std::any_of(entries.begin(), entries.end(), [&](const auto& entry) {
                return entry.format() == "CLAP" && entry.bundlePath() == bundle;
            }))
                continue;
            std::string error;
            scanner->scanBundle(bundle, false, 0.0, [&](AudioPluginCatalogEntry entry) {
                if (std::none_of(entries.begin(), entries.end(), [&](const auto& existing) {
                    return existing.format() == entry.format() && existing.pluginId() == entry.pluginId();
                })) {
                    entries.push_back(std::move(entry));
                    changed = true;
                }
            }, [&](std::string message) { error = std::move(message); });
            if (!error.empty())
                remidy::Logger::global()->logWarning("CLAP scan: %s", error.c_str());
        }
    }
    if (changed && !tool->pluginListCacheFile().empty()) {
        AudioPluginCatalog catalog;
        for (const auto& entry : entries)
            if (entry.format() == "CLAP" || entry.format() == "VST3")
                catalog.add(entry);
        catalog.save(tool->pluginListCacheFile());
    }
}

#if _WIN32
namespace {
struct Module {
    HMODULE handle;
    bool alreadyLoaded;
    explicit Module(const std::filesystem::path& path)
        : handle(nullptr), alreadyLoaded(GetModuleHandleW(path.c_str()) != nullptr) {
        handle = LoadLibraryW(path.c_str());
    }
    ~Module() { if (handle) FreeLibrary(handle); }
};

void clapKinds(const std::filesystem::path& path, PluginKinds& kinds) {
    Module module(path);
    if (!module.handle) return;
    const auto* entry = reinterpret_cast<const clap_plugin_entry_t*>(GetProcAddress(module.handle, "clap_entry"));
    if (!entry || !entry->init || !entry->deinit || !entry->get_factory) return;
    const auto utf8 = path.u8string();
    const std::string filename(utf8.begin(), utf8.end());
    if (!entry->init(filename.c_str())) return;
    const auto* factory = static_cast<const clap_plugin_factory_t*>(entry->get_factory(CLAP_PLUGIN_FACTORY_ID));
    if (factory) {
        for (uint32_t i = 0; i < factory->get_plugin_count(factory); ++i) {
            const auto* descriptor = factory->get_plugin_descriptor(factory, i);
            if (!descriptor || !descriptor->id || !descriptor->features) continue;
            bool instrument = false, effect = false;
            for (auto features = descriptor->features; *features; ++features) {
                instrument |= std::strcmp(*features, "instrument") == 0;
                effect |= std::strcmp(*features, "audio-effect") == 0;
            }
            if (instrument != effect)
                kinds[{"CLAP", descriptor->id}] = instrument ? "instrument" : "effect";
        }
    }
    entry->deinit();
}

void vstKinds(const std::filesystem::path& bundle, PluginKinds& kinds) {
    auto path = bundle;
    if (std::filesystem::is_directory(path))
        path = uh::windowsVst3Binary(bundle, bundle / "Contents" / "x86_64-win");
    if (path.empty() || !uh::isVst3Binary(path)) return;
    Module module(path);
    if (!module.handle) return;
    auto init = reinterpret_cast<bool (*)()>(GetProcAddress(module.handle, "InitDll"));
    auto exit = reinterpret_cast<bool (*)()>(GetProcAddress(module.handle, "ExitDll"));
    if (!module.alreadyLoaded && init && !init()) return;
    auto getFactory = reinterpret_cast<Steinberg::IPluginFactory* (*)()>(GetProcAddress(module.handle, "GetPluginFactory"));
    auto* factory = getFactory ? getFactory() : nullptr;
    Steinberg::IPluginFactory2* factory2 = nullptr;
    if (factory && factory->queryInterface(Steinberg::IPluginFactory2::iid, reinterpret_cast<void**>(&factory2)) == Steinberg::kResultOk) {
        for (int i = 0; i < factory2->countClasses(); ++i) {
            Steinberg::PClassInfo2 info{};
            if (factory2->getClassInfo2(i, &info) != Steinberg::kResultOk ||
                std::strcmp(info.category, kVstAudioEffectClass) != 0) continue;
            char id[33]{};
            Steinberg::FUID::fromTUID(info.cid).toString(id);
            const std::string categories(info.subCategories);
            bool instrument = false, effect = false;
            size_t start = 0;
            do {
                const auto end = categories.find('|', start);
                const auto token = categories.substr(start, end - start);
                instrument |= token == "Instrument";
                effect |= token == "Fx";
                if (end == std::string::npos) break;
                start = end + 1;
            } while (start < categories.size());
            if (instrument != effect)
                kinds[{"VST3", id}] = instrument ? "instrument" : "effect";
        }
        factory2->release();
    }
    if (factory) factory->release();
    if (!module.alreadyLoaded && exit) exit();
}
}
#endif

PluginKinds readPluginKinds(const std::vector<AudioPluginCatalogEntry>& entries) {
    PluginKinds kinds;
#if _WIN32
    std::set<std::pair<std::string, std::filesystem::path>> bundles;
    for (const auto& entry : entries)
        bundles.emplace(entry.format(), entry.bundlePath());
    for (const auto& [format, path] : bundles) {
        try {
            if (format == "CLAP") clapKinds(path, kinds);
            if (format == "VST3") vstKinds(path, kinds);
        } catch (...) {
            // Missing or conflicting categories remain explicitly unknown.
        }
    }
#endif
    return kinds;
}
