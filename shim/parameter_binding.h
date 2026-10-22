#pragma once
#include "../patches/uapmd/parameter-automation/backend_api.h"
#include <vector>
#include <cmath>

struct ParameterDefinition { double min{}, max{}; bool eligible{}; uh::automation::Target target; };
struct ParameterBinding {
    OwnedInstance* owned{};
    remidy::PluginParameterSupport* support{};
    uint64_t generation{}, revision{};
    bool normalized{}, prepared{}, pending{};
    std::vector<ParameterDefinition> definitions;
    int (*enqueue)(remidy::PluginParameterSupport*, uh::automation::Target, double){};
    int (*note)(remidy::PluginParameterSupport*, uint32_t){};
    void (*cancel)(remidy::PluginParameterSupport*){};

    bool valid() const {
        return prepared && owned && generation == owned->parameter_generation
            && revision == owned->parameter_list_changes.load(std::memory_order_acquire);
    }
    void clear() { if (cancel && support) cancel(support); pending = false; }
};
