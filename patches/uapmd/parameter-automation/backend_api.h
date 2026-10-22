#pragma once
#include <cstdint>
#include <string>
#include <atomic>
namespace remidy { class PluginParameterSupport; }
namespace uh::automation {
struct Target { uint32_t id{}; void* cookie{}; std::atomic<double>* current{}; };
bool clapResolve(remidy::PluginParameterSupport*, uint32_t, Target*);
bool vst3Resolve(remidy::PluginParameterSupport*, uint32_t, Target*);
int clapApply(remidy::PluginParameterSupport*, Target, double);
int vst3Apply(remidy::PluginParameterSupport*, Target, double);
int clapNote(remidy::PluginParameterSupport*, uint32_t);
int vst3Note(remidy::PluginParameterSupport*, uint32_t);
// UI-only prepare/get/text; enqueue/cancel are allocation-free audio-thread calls.
bool clapPrepare(remidy::PluginParameterSupport*);
void clapCancel(remidy::PluginParameterSupport*);
bool vst3Prepare(remidy::PluginParameterSupport*);
void vst3Cancel(remidy::PluginParameterSupport*);
int vst3Current(remidy::PluginParameterSupport*, uint32_t, double*);
std::string vst3Text(remidy::PluginParameterSupport*, uint32_t, double);
}
