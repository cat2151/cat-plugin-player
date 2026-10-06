#pragma once

#include <uapmd-plugin-hosting/uapmd-plugin-hosting.hpp>
#include <stdexcept>

// Each live instance owns its metadata and format objects. A background scan
// cannot invalidate its catalog entry or mutate its format's entry cache.
struct OwnedInstance {
    std::unique_ptr<uapmd_plugin_hosting::PluginScanTool> tool;
    uapmd_plugin_hosting::AudioPluginCatalogEntry entry;
    std::unique_ptr<uapmd_plugin_hosting::PluginInstancing> lifecycle;

    OwnedInstance(const uapmd_plugin_hosting::AudioPluginCatalogEntry& source,
                  uint32_t sampleRate, uint32_t blockSize)
        : tool(uapmd_plugin_hosting::PluginScanTool::create()), entry(source) {
        for (auto* format : tool->formats()) {
            if (format->name() != entry.format())
                continue;
            lifecycle = std::make_unique<uapmd_plugin_hosting::PluginInstancing>(
                *tool, format, &entry);
            auto& config = lifecycle->configurationRequest();
            config.sampleRate = sampleRate;
            config.bufferSizeInSamples = blockSize;
            config.offlineMode = false;
            return;
        }
        throw std::runtime_error("Plugin format not found: " + entry.format());
    }
};
