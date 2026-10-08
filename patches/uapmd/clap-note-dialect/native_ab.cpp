// Independent direct CLAP host. Each invocation loads one fresh instance.
#include <windows.h>
#include <clap/clap.h>
#include <algorithm>
#include <cmath>
#include <cstring>
#include <fstream>
#include <iostream>
#include <map>
#include <vector>
#include <stdexcept>
#include "note_dialect.h"

static void require(bool ok, const char* message) { if (!ok) throw std::runtime_error(message); }
static bool audioThread = false;
static const DWORD mainThread = GetCurrentThreadId();
static const void* extension(const clap_host_t*, const char* name) {
    static clap_host_thread_check_t check{
        [](const clap_host_t*) { return !audioThread && GetCurrentThreadId() == mainThread; },
        [](const clap_host_t*) { return audioThread && GetCurrentThreadId() == mainThread; }};
    return std::strcmp(name, CLAP_EXT_THREAD_CHECK) == 0 ? &check : nullptr;
}
struct State { std::vector<char> data; size_t position{}; };
struct Events { std::vector<const clap_event_header_t*> items; };
struct Meter {
    double sum{}, peak{}; uint64_t count{};
    void add(float value) { require(std::isfinite(value), "nonfinite output"); sum += double(value) * value; peak = std::max(peak, std::abs(double(value))); ++count; }
    void print(const char* name) { std::cout << name << " rms=" << std::sqrt(sum / std::max(uint64_t(1), count)) << " peak=" << peak << '\n'; }
};
int main(int argc, char** argv) {
    try {
        require(argc == 5, "usage: native_ab plugin state native|midi off|held");
        State state;
        if (std::strcmp(argv[2], "-") != 0) {
            std::ifstream file(argv[2], std::ios::binary);
            require(bool(file), "state open failed");
            state.data.assign(std::istreambuf_iterator<char>(file), {});
        }
        auto module = LoadLibraryA(argv[1]); require(module, "LoadLibrary failed");
        auto* entry = reinterpret_cast<const clap_plugin_entry_t*>(GetProcAddress(module, "clap_entry"));
        require(entry && entry->init(argv[1]), "entry init failed");
        auto* factory = static_cast<const clap_plugin_factory_t*>(entry->get_factory(CLAP_PLUGIN_FACTORY_ID));
        require(factory && factory->get_plugin_count(factory), "factory missing");
        clap_host_t host{CLAP_VERSION, nullptr, "cat direct A/B", "cat", "", "1", extension,
            [](const clap_host_t*) {}, [](const clap_host_t*) {}, [](const clap_host_t*) {}};
        const auto* plugin = factory->create_plugin(factory, &host, factory->get_plugin_descriptor(factory, 0)->id);
        require(plugin && plugin->init(plugin), "plugin init failed");
        const auto* ports = static_cast<const clap_plugin_note_ports_t*>(plugin->get_extension(plugin, CLAP_EXT_NOTE_PORTS));
        clap_note_port_info_t port{};
        require(ports && ports->get(plugin, 0, true, &port), "input note port missing");
        std::cout << "plugin=" << plugin->desc->name << " id=" << plugin->desc->id << " version=" << plugin->desc->version
                  << " supported=" << port.supported_dialects << " preferred=" << port.preferred_dialect
                  << " state_bytes=" << state.data.size() << " rate=48000 block=256 key=60 velocity=100 channel=0 CC1=0 on_sample=256 off_sample=48128 arm=" << argv[3] << " off=" << argv[4] << '\n';
        if (std::strcmp(argv[4], "inspect") == 0) {
            plugin->destroy(plugin); entry->deinit(); FreeLibrary(module); return 0;
        }
        auto* states = static_cast<const clap_plugin_state_t*>(plugin->get_extension(plugin, CLAP_EXT_STATE));
        clap_istream_t stream{&state, [](const clap_istream_t* s, void* dst, uint64_t size) -> int64_t {
            auto& context = *static_cast<State*>(s->ctx);
            size = std::min(size, uint64_t(context.data.size() - context.position));
            std::memcpy(dst, context.data.data() + context.position, size); context.position += size; return size;
        }};
        if (!state.data.empty()) require(states && states->load(plugin, &stream), "state load failed");
        auto* audio = static_cast<const clap_plugin_audio_ports_t*>(plugin->get_extension(plugin, CLAP_EXT_AUDIO_PORTS));
        require(audio && audio->count(plugin, true) == 0 && audio->count(plugin, false) == 1, "expected one output/no input");
        clap_audio_port_info_t audioInfo{};
        require(audio->get(plugin, 0, false, &audioInfo), "audio port get failed");
        std::vector<std::vector<float>> samples(audioInfo.channel_count, std::vector<float>(256));
        std::vector<float*> channels; for (auto& channel : samples) channels.push_back(channel.data());
        clap_audio_buffer_t buffer{channels.data(), nullptr, audioInfo.channel_count, 0, 0};
        Events events;
        clap_input_events_t input{&events,
            [](const clap_input_events_t* e) -> uint32_t { return static_cast<Events*>(e->ctx)->items.size(); },
            [](const clap_input_events_t* e, uint32_t i) { return static_cast<Events*>(e->ctx)->items.at(i); }};
        clap_output_events_t output{nullptr, [](const clap_output_events_t*, const clap_event_header_t*) { return true; }};
        require(plugin->activate(plugin, 48000, 256, 256), "activate failed");
        audioThread = true;
        require(plugin->start_processing(plugin), "start failed");
        Meter sounding, earlyTail, lateTail; std::map<int, int> statuses;
        alignas(8) unsigned char note[128]{}, cc[128]{};
        const uint32_t dialect = std::strcmp(argv[3], "native") == 0 ? 1 : 2;
        const bool off = std::strcmp(argv[4], "off") == 0;
        for (int block = 0; block < 1500; ++block) {
            events.items.clear();
            if (block == 0) {
                uh::clap_dialect::cc([&](size_t) -> void* { return cc; }, 2, 0, 0, 1, 0, 0);
                events.items.push_back(reinterpret_cast<clap_event_header_t*>(cc));
            }
            if (block == 1 || (block == 188 && off)) {
                uh::clap_dialect::note([&](size_t) -> void* { return note; }, dialect, block == 1, 0, 0, 60, 100 << 9, 0);
                events.items.push_back(reinterpret_cast<clap_event_header_t*>(note));
            }
            for (auto& channel : samples) std::fill(channel.begin(), channel.end(), 0.0f);
            clap_process_t process{int64_t(block) * 256, 256, nullptr, nullptr, &buffer, 0, 1, &input, &output};
            int status = plugin->process(plugin, &process); ++statuses[status]; require(status != CLAP_PROCESS_ERROR, "process error");
            if (block >= 1 && block < 188) for (auto& channel : samples) for (float value : channel) sounding.add(value);
            if (block >= 188 && block < 376) for (auto& channel : samples) for (float value : channel) earlyTail.add(value);
            if (block >= 1125) for (auto& channel : samples) for (float value : channel) lateTail.add(value);
        }
        sounding.print("sounding"); earlyTail.print("early_tail"); lateTail.print("late_tail");
        std::cout << "status_counts"; for (auto [status, count] : statuses) std::cout << ' ' << status << ':' << count; std::cout << '\n';
        plugin->stop_processing(plugin); audioThread = false; plugin->deactivate(plugin); plugin->destroy(plugin); entry->deinit(); FreeLibrary(module);
    } catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
