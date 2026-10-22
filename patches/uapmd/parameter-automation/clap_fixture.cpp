// Test-only instrument. Audio channels expose processor-observed parameter values.
#include <clap/clap.h>
#include <cstring>
#include <cstdio>

static const char* features[]{CLAP_PLUGIN_FEATURE_INSTRUMENT, nullptr};
static const clap_plugin_descriptor_t descriptor{
    CLAP_VERSION, "cat.test.parameters", "Parameter fixture", "cat test", "", "", "", "1", "", features};
struct Fixture {
    clap_plugin_t plugin{};
    double values[5]{0.2, 0.3, 0.4, 0.5, 0.6};
    bool invalid{};
    const clap_host_t* host{};
    bool rescan{};
};
static Fixture& data(const clap_plugin_t* p) { return *static_cast<Fixture*>(p->plugin_data); }
static const void* extension(const clap_plugin_t*, const char* id) {
    static const clap_plugin_state_t state{
        [](const clap_plugin_t* p,const clap_ostream_t* stream) {
            return stream->write(stream,data(p).values,sizeof(double)*2)==sizeof(double)*2;
        },
        [](const clap_plugin_t* p,const clap_istream_t* stream) {
            double values[2];
            if (stream->read(stream,values,sizeof(values))!=sizeof(values)) return false;
            std::memcpy(data(p).values,values,sizeof(values));data(p).invalid=false;data(p).rescan=false;return true;
        }};
    static const clap_plugin_audio_ports_t audio{
        [](const clap_plugin_t*, bool input) -> uint32_t { return input ? 0 : 1; },
        [](const clap_plugin_t*, uint32_t i, bool input, clap_audio_port_info_t* info) {
            if (i || input) return false;
            *info = {}; info->id = 0; info->channel_count = 2;
            info->flags = CLAP_AUDIO_PORT_IS_MAIN; info->port_type = CLAP_PORT_STEREO;
            info->in_place_pair = CLAP_INVALID_ID; return true;
        }};
    static const clap_plugin_note_ports_t notes{
        [](const clap_plugin_t*, bool input) -> uint32_t { return input ? 1 : 0; },
        [](const clap_plugin_t*, uint32_t i, bool input, clap_note_port_info_t* info) {
            if (i || !input) return false;
            *info = {}; info->id = 0; info->supported_dialects = CLAP_NOTE_DIALECT_CLAP;
            info->preferred_dialect = CLAP_NOTE_DIALECT_CLAP; return true;
        }};
    static const clap_plugin_params_t params{
        [](const clap_plugin_t*) -> uint32_t { return 5; },
        [](const clap_plugin_t*, uint32_t i, clap_param_info_t* info) {
            if (i >= 5) return false;
            *info = {}; info->id = 101 + i * 17; info->min_value = 0; info->max_value = 1;
            info->default_value = 0.2 + i * 0.1;
            info->flags = i == 2 ? 0 : CLAP_PARAM_IS_AUTOMATABLE;
            if (i == 3) info->flags |= CLAP_PARAM_IS_READONLY;
            if (i == 4) info->flags |= CLAP_PARAM_IS_HIDDEN;
            std::snprintf(info->name, sizeof(info->name), "Same name");
            std::snprintf(info->module, sizeof(info->module), "Module/%u", i); return true;
        },
        [](const clap_plugin_t* p, clap_id id, double* value) {
            if (id < 101 || (id - 101) % 17 || (id - 101) / 17 >= 5) return false;
            *value = data(p).values[(id - 101) / 17]; return true;
        },
        [](const clap_plugin_t*, clap_id, double value, char* text, uint32_t size) {
            std::snprintf(text, size, "native %.3f", value); return true;
        }, [](const clap_plugin_t*, clap_id, const char*, double*) { return false; },
        [](const clap_plugin_t*, const clap_input_events_t*, const clap_output_events_t*) {}};
    if (!std::strcmp(id, CLAP_EXT_AUDIO_PORTS)) return &audio;
    if (!std::strcmp(id, CLAP_EXT_NOTE_PORTS)) return &notes;
    if (!std::strcmp(id, CLAP_EXT_PARAMS)) return &params;
    if (!std::strcmp(id, CLAP_EXT_STATE)) return &state;
    return nullptr;
}
static clap_process_status process(const clap_plugin_t* p, const clap_process_t* block) {
    auto& f = data(p);
    uint32_t previous = 0;
    for (uint32_t i = 0; i < block->in_events->size(block->in_events); ++i) {
        auto* event = block->in_events->get(block->in_events, i);
        f.invalid |= event->time < previous; previous = event->time;
        if (event->type == CLAP_EVENT_PARAM_VALUE) {
            auto& v = *reinterpret_cast<const clap_event_param_value_t*>(event);
            f.invalid |= v.note_id != -1 || v.port_index != -1 || v.channel != -1 || v.key != -1;
            f.invalid |= (event->flags & CLAP_EVENT_IS_LIVE) != 0;
            if (v.param_id == 101) f.values[0] = v.value;
            else if (v.param_id == 118) f.values[1] = v.value;
            else f.invalid = true;
        } else if (event->type == CLAP_EVENT_NOTE_ON) {
            // Both parameter updates must reach the processor before this note.
            auto& note=*reinterpret_cast<const clap_event_note_t*>(event);
            if (note.key==60) f.invalid |= f.values[0] != 0.7 || f.values[1] != 0.8;
            if (note.key==61) f.invalid |= f.values[0] != 0.2 || f.values[1] != 0.3;
        }
    }
    for (uint32_t c = 0; c < 2; ++c)
        for (uint32_t i = 0; i < block->frames_count; ++i)
            block->audio_outputs[0].data32[c][i] = f.invalid ? -1.f : float(f.values[c]);
    if (f.values[0] == 0.91 && !f.rescan) {
        f.rescan = true;
        f.host->request_callback(f.host);
    }
    for (uint32_t i = 0; i < 2; ++i) {
        clap_event_param_value_t event{};
        event.header = {sizeof(event), 0, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_PARAM_VALUE, 0};
        event.param_id = 101 + i * 17; event.value = f.values[i];
        event.note_id = -1; event.port_index = -1; event.channel = -1; event.key = -1;
        if (!block->out_events->try_push(block->out_events, &event.header)) return CLAP_PROCESS_ERROR;
    }
    return CLAP_PROCESS_CONTINUE;
}
static const clap_plugin_t* create(const clap_plugin_factory_t*, const clap_host_t* host, const char* id) {
    if (std::strcmp(id, descriptor.id)) return nullptr;
    auto* f = new Fixture;
    f->host = host;
    f->plugin = {&descriptor, f, [](const clap_plugin_t*) { return true; },
        [](const clap_plugin_t* p) { delete &data(p); },
        [](const clap_plugin_t*, double, uint32_t, uint32_t) { return true; },
        [](const clap_plugin_t*) {}, [](const clap_plugin_t*) { return true; },
        [](const clap_plugin_t*) {}, [](const clap_plugin_t*) {}, process, extension,
        [](const clap_plugin_t* p) {
            auto& f = data(p);
            if (f.rescan) {
                auto* params = static_cast<const clap_host_params_t*>(f.host->get_extension(f.host, CLAP_EXT_PARAMS));
                if (params) params->rescan(f.host, CLAP_PARAM_RESCAN_INFO);
            }
        }};
    return &f->plugin;
}
static const clap_plugin_factory_t factory{
    [](const clap_plugin_factory_t*) -> uint32_t { return 1; },
    [](const clap_plugin_factory_t*, uint32_t i) { return i ? nullptr : &descriptor; }, create};
extern "C" CLAP_EXPORT const clap_plugin_entry_t clap_entry{
    CLAP_VERSION, [](const char*) { return true; }, [] {},
    [](const char* id) -> const void* { return !std::strcmp(id, CLAP_PLUGIN_FACTORY_ID) ? &factory : nullptr; }};
