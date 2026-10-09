// Isolated fixture, addressed by bundle path; never install or register it.
#include <clap/clap.h>
#include <clap/ext/state-context.h>
#include <cstring>
#include <stdexcept>

static const char* features[]{CLAP_PLUGIN_FEATURE_INSTRUMENT, nullptr};
static const char* effect_features[]{CLAP_PLUGIN_FEATURE_AUDIO_EFFECT, nullptr};
static const clap_plugin_descriptor_t descriptors[]{
    {CLAP_VERSION, "cat.test.state.normal", "State normal", "cat test", "", "", "", "1", "", features},
    {CLAP_VERSION, "cat.test.state.context", "State context", "cat test", "", "", "", "1", "", features},
    {CLAP_VERSION, "cat.test.state.none", "State absent", "cat test", "", "", "", "1", "", features},
    {CLAP_VERSION, "cat.test.state.effect", "State effect", "cat test", "", "", "", "1", "", effect_features}};
struct Fixture {
    clap_plugin_t plugin{};
    uint8_t value{1};
    bool context{};
    bool absent{};
    bool effect{};
};
static Fixture& data(const clap_plugin_t* p) { return *static_cast<Fixture*>(p->plugin_data); }
static bool save(const clap_plugin_t* p, const clap_ostream_t* stream) {
    return stream->write(stream, &data(p).value, 1) == 1;
}
static bool load(const clap_plugin_t* p, const clap_istream_t* stream) {
    uint8_t value{};
    if (stream->read(stream, &value, 1) != 1 || value == 0) return false;
    if (value == 2) throw std::runtime_error("fixture load exception");
    if (value == 3) throw 3;
    data(p).value = value;
    return true;
}
static const void* extension(const clap_plugin_t* p, const char* id) {
    // Context descriptor must exercise context.load rather than accidentally
    // passing through the normal extension with indistinguishable results.
    static const clap_plugin_state_t normal{save,
        [](const clap_plugin_t* p, const clap_istream_t* s) {
            return !data(p).context && load(p, s);
        }};
    static const clap_plugin_state_context_t context{
        [](const clap_plugin_t* p, const clap_ostream_t* s, uint32_t) { return save(p, s); },
        [](const clap_plugin_t* p, const clap_istream_t* s, uint32_t type) {
            if (type != CLAP_STATE_CONTEXT_FOR_PROJECT) return false;
            return load(p, s);
        }};
    static const clap_plugin_audio_ports_t audio{
        [](const clap_plugin_t* p, bool input) -> uint32_t { return input && !data(p).effect ? 0 : 1; },
        [](const clap_plugin_t* p, uint32_t index, bool input, clap_audio_port_info_t* info) {
            if (index || (input && !data(p).effect)) return false;
            *info = {};
            info->id = 0; info->channel_count = 2; info->flags = CLAP_AUDIO_PORT_IS_MAIN;
            info->port_type = CLAP_PORT_STEREO; info->in_place_pair = CLAP_INVALID_ID;
            return true;
        }};
    if (!std::strcmp(id, CLAP_EXT_AUDIO_PORTS)) return &audio;
    if (data(p).absent) return nullptr;
    if (data(p).context && !std::strcmp(id, CLAP_EXT_STATE_CONTEXT)) return &context;
    if (!std::strcmp(id, CLAP_EXT_STATE)) return &normal;
    return nullptr;
}
static const clap_plugin_t* create(const clap_plugin_factory_t*, const clap_host_t*, const char* id) {
    for (uint32_t i = 0; i < 4; ++i) {
        if (std::strcmp(id, descriptors[i].id)) continue;
        auto* f = new Fixture;
        f->context = i == 1; f->absent = i == 2; f->effect = i == 3;
        f->plugin = {&descriptors[i], f,
            [](const clap_plugin_t*) { return true; },
            [](const clap_plugin_t* p) { delete &data(p); },
            [](const clap_plugin_t*, double, uint32_t, uint32_t) { return true; },
            [](const clap_plugin_t*) {}, [](const clap_plugin_t*) { return true; },
            [](const clap_plugin_t*) {}, [](const clap_plugin_t*) {},
            [](const clap_plugin_t*, const clap_process_t* process) -> clap_process_status {
                for (uint32_t b = 0; b < process->audio_outputs_count; ++b)
                    for (uint32_t c = 0; c < process->audio_outputs[b].channel_count; ++c)
                        if (process->audio_outputs[b].data32)
                            std::memset(process->audio_outputs[b].data32[c], 0, sizeof(float) * process->frames_count);
                return CLAP_PROCESS_CONTINUE;
            }, extension, [](const clap_plugin_t*) {}};
        return &f->plugin;
    }
    return nullptr;
}
static const clap_plugin_factory_t factory{
    [](const clap_plugin_factory_t*) -> uint32_t { return 4; },
    [](const clap_plugin_factory_t*, uint32_t i) { return i < 4 ? &descriptors[i] : nullptr; }, create};
extern "C" CLAP_EXPORT const clap_plugin_entry_t clap_entry{
    CLAP_VERSION, [](const char*) { return true; }, [] {},
    [](const char* id) -> const void* { return !std::strcmp(id, CLAP_PLUGIN_FACTORY_ID) ? &factory : nullptr; }};
