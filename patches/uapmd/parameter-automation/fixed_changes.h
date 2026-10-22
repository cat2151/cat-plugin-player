#pragma once
#include <array>
#include <pluginterfaces/vst/ivstparameterchanges.h>
namespace uh::automation {
using namespace Steinberg;
using namespace Steinberg::Vst;
// Separate from remidy's live edit queues. Constructed with the instance, never
// allocates/reclaims storage during process. Bounded merging preserves live edits.
class FixedQueue final : public IParamValueQueue {
    struct Point { int32 offset{}; double value{}; };
    std::array<Point, 64> points{};
    int32 size{};
    ParamID id{};
    bool failed{};
public:
    void reset(ParamID value) { id = value; size = 0; failed = false; }
    bool overflowed() const { return failed; }
    tresult PLUGIN_API queryInterface(const TUID, void** obj) override { *obj = nullptr; return kNoInterface; }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    ParamID PLUGIN_API getParameterId() override { return id; }
    int32 PLUGIN_API getPointCount() override { return size; }
    tresult PLUGIN_API getPoint(int32 i, int32& offset, ParamValue& value) override {
        if (i < 0 || i >= size) return kInvalidArgument;
        offset = points[i].offset; value = points[i].value; return kResultOk;
    }
    tresult PLUGIN_API addPoint(int32 offset, ParamValue value, int32& i) override {
        if (size == points.size()) { failed = true; return kOutOfMemory; }
        i = size; points[size++] = {offset, value}; return kResultOk;
    }
};
class FixedChanges final : public IParameterChanges {
    std::array<FixedQueue, 256> queues{};
    int32 size{};
    bool failed{};
public:
    void clear() { size = 0; failed = false; }
    bool overflowed() const {
        if (failed) return true;
        for (int32 i = 0; i < size; ++i) if (queues[i].overflowed()) return true;
        return false;
    }
    tresult PLUGIN_API queryInterface(const TUID, void** obj) override { *obj = nullptr; return kNoInterface; }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    int32 PLUGIN_API getParameterCount() override { return size; }
    IParamValueQueue* PLUGIN_API getParameterData(int32 i) override { return i >= 0 && i < size ? &queues[i] : nullptr; }
    IParamValueQueue* PLUGIN_API addParameterData(const ParamID& id, int32& i) override {
        for (i = 0; i < size; ++i) if (queues[i].getParameterId() == id) return &queues[i];
        if (size == queues.size()) { failed = true; return nullptr; }
        queues[size].reset(id); return &queues[size++];
    }
    bool append(IParameterChanges& input) {
        for (int32 i = 0; i < input.getParameterCount(); ++i) {
            auto* source = input.getParameterData(i); int32 index;
            auto* dest = addParameterData(source->getParameterId(), index);
            if (!dest) return false;
            for (int32 j = 0; j < source->getPointCount(); ++j) {
                int32 offset; double value;
                if (source->getPoint(j, offset, value) != kResultOk || dest->addPoint(offset, value, index) != kResultOk)
                    return false;
            }
        }
        return true;
    }
};
}
