#pragma once
#include <uapmd-plugin-hosting/uapmd-plugin-hosting.hpp>
#include <map>

using PluginKinds = std::map<std::pair<std::string, std::string>, std::string>;
// Runs on the scan worker; no plugin instances are created.
void completeClapCatalog(uapmd_plugin_hosting::AudioPluginHostingAPI& api,
                         std::vector<uapmd_plugin_hosting::AudioPluginCatalogEntry>& entries);
PluginKinds readPluginKinds(const std::vector<uapmd_plugin_hosting::AudioPluginCatalogEntry>& entries);
