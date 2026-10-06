// uapmd_shim.cpp - see uapmd_shim.h for the threading rules.
//
// No C++ exception may cross the C boundary, so every entry point catches everything.

#include "uapmd_shim.h"
#include "owned_instance.h"
#if _WIN32
#include <Windows.h>
#endif

#include <algorithm>
#include <atomic>
#include <chrono>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <queue>
#include <set>
#include <string>
#include <thread>
#include <vector>
#include <cstring>

#include <uapmd-plugin-hosting/uapmd-plugin-hosting.hpp>
#include <remidy-gui/remidy-gui.hpp>

using uapmd_plugin_hosting::AudioPluginCatalogEntry;
using uapmd_plugin_hosting::AudioPluginHostingAPI;

namespace {

// remidy sends "run this on the UI thread" requests to an EventLoop. The UI thread
// is shared by startup and egui; uh_pump() runs the queued work.
class ShimEventLoop final : public remidy::EventLoop {
public:
    void setWake(UhWakeFn wake, void* user) {
        std::lock_guard lock(wake_mutex_);
        wake_ = wake;
        wake_user_ = user;
    }

    void post(std::function<void()>&& task) {
        {
            std::lock_guard lock(mutex_);
            tasks_.push(std::move(task));
        }
        UhWakeFn wake;
        void* user;
        {
            std::lock_guard lock(wake_mutex_);
            wake = wake_;
            user = wake_user_;
        }
        if (wake)
            wake(user);
    }

    void drain() {
        std::queue<std::function<void()>> tasks;
        {
            std::lock_guard lock(mutex_);
            std::swap(tasks, tasks_);
        }
        while (!tasks.empty()) {
            auto task = std::move(tasks.front());
            tasks.pop();
            if (task)
                task();
        }
    }

protected:
    void initializeOnUIThreadImpl() override {}
    bool runningOnMainThreadImpl() override { return std::this_thread::get_id() == main_thread_; }
    void enqueueTaskOnMainThreadImpl(std::function<void()>&& task) override { post(std::move(task)); }
    void startImpl() override {}
    void stopImpl() override {}
    void processQueuedTasksImpl() override { drain(); }

private:
    std::thread::id main_thread_{std::this_thread::get_id()};
    std::mutex mutex_;
    std::queue<std::function<void()>> tasks_;
    std::mutex wake_mutex_;
    UhWakeFn wake_{nullptr};
    void* wake_user_{nullptr};
};

// remidy keeps a raw pointer to the event loop, so it lives for the whole process.
ShimEventLoop* g_loop = nullptr;

} // namespace

struct UhHost {
    // Used only for scanning (and main-thread COM initialization).
    std::unique_ptr<AudioPluginHostingAPI> api;
    std::vector<AudioPluginCatalogEntry> catalog;
    std::map<int32_t, std::unique_ptr<OwnedInstance>> instances;
    int32_t next_instance_id{0};
    int32_t pending_instances{0};
    std::map<int32_t, std::unique_ptr<remidy::gui::ContainerWindow>> windows;
    std::set<int32_t> ui_created;
    std::thread scan_thread;
    std::atomic<bool> scanning{false};
};

// Everything one plugin instance needs on the audio thread.
struct UhProcessor {
    uapmd_plugin_hosting::AudioPluginInstanceAPI* instance{};
    remidy::MasterContext master{};
    remidy::AudioProcessContext ctx;
    uint32_t max_frames{};

    explicit UhProcessor(uint32_t eventBufferBytes) : ctx(master, eventBufferBytes) {}
};

namespace {

constexpr uint32_t kEventBufferBytes = 4096;

uapmd_plugin_hosting::AudioPluginInstanceAPI* getInstance(UhHost* host, int32_t id) {
    auto it = host->instances.find(id);
    return it == host->instances.end() ? nullptr : it->second->lifecycle->instance();
}

const std::string* pluginField(const AudioPluginCatalogEntry& e, int32_t field) {
    switch (field) {
        case UH_PLUGIN_NAME:   return &e.displayName();
        case UH_PLUGIN_VENDOR: return &e.vendorName();
        case UH_PLUGIN_FORMAT: return &e.format();
        case UH_PLUGIN_ID:     return &e.pluginId();
        default:               return nullptr;
    }
}

void destroyInstance(UhHost* host, int32_t instanceId) {
    // The instance tears its editor down while the container window still exists;
    // the container goes afterwards.
    {
        // Destroying an instance stops its processing, which CLAP wants done in the
        // audio-thread role. The caller has already stopped the audio stream (see
        // uapmd_shim.h), so no audio thread exists and this thread may take the role.
        remidy::AudioThreadScope audioThreadRole;
        host->instances.erase(instanceId);
    }
    host->ui_created.erase(instanceId);
    host->windows.erase(instanceId);
}

} // namespace

extern "C" {

UhHost* uh_create(UhWakeFn wake, void* wake_user) {
    try {
        if (!g_loop)
            g_loop = new ShimEventLoop();
        g_loop->setWake(wake, wake_user);
        remidy::setEventLoop(g_loop);
        remidy::EventLoop::initializeOnUIThread();

        auto host = std::make_unique<UhHost>();
        host->api = AudioPluginHostingAPI::create();
        if (!host->api)
            return nullptr;
        return host.release();
    } catch (...) {
        return nullptr;
    }
}

void uh_destroy(UhHost* host) {
    if (!host)
        return;
    try {
        // A scan cannot be cancelled here. Its worker may be waiting for the main
        // thread, so keep pumping until it is done.
        while (host->scanning.load() || host->pending_instances != 0) {
            uh_pump_startup(host);
            std::this_thread::sleep_for(std::chrono::milliseconds(5));
        }
        if (host->scan_thread.joinable())
            host->scan_thread.join();
        g_loop->drain();

        while (!host->instances.empty())
            destroyInstance(host, host->instances.begin()->first);
        host->windows.clear();
        host->api.reset();
    } catch (...) {
    }
    if (g_loop)
        g_loop->setWake(nullptr, nullptr);
    delete host;
}

void uh_pump(UhHost* host) {
    if (!host || !g_loop)
        return;
    try {
        g_loop->drain();
    } catch (...) {
    }
}

void uh_pump_startup(UhHost* host) {
    uh_pump(host);
#if _WIN32
    MSG message;
    while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
        if (message.message == WM_QUIT) {
            PostQuitMessage(static_cast<int>(message.wParam));
            break;
        }
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
#endif
}

int32_t uh_scan_async(UhHost* host, int32_t rescan, UhScanDoneFn done, void* user) {
    if (!host)
        return -1;
    try {
        if (host->scanning.exchange(true))
            return -1;
        if (host->scan_thread.joinable())
            host->scan_thread.join();

        host->scan_thread = std::thread([host, rescan, done, user] {
            std::string error;
            try {
                host->api->performPluginScanning(rescan != 0);
            } catch (const std::exception& e) {
                error = e.what();
                if (error.empty())
                    error = "plugin scanning failed";
            } catch (...) {
                error = "plugin scanning failed";
            }
            g_loop->post([host, done, user, error] {
                try {
                    host->catalog = host->api->pluginCatalogEntries();
                } catch (...) {
                    host->catalog.clear();
                }
                host->scanning.store(false);
                if (done)
                    done(user, error.empty() ? nullptr : error.c_str());
            });
        });
        return 0;
    } catch (...) {
        host->scanning.store(false);
        return -1;
    }
}

int32_t uh_plugin_count(UhHost* host) {
    return host ? static_cast<int32_t>(host->catalog.size()) : 0;
}

int32_t uh_plugin_info(UhHost* host, int32_t index, int32_t field, char* buf, int32_t buf_len) {
    if (!host || index < 0 || static_cast<size_t>(index) >= host->catalog.size())
        return -1;
    try {
        const auto& entry = host->catalog[static_cast<size_t>(index)];
        std::string path;
        if (field == UH_PLUGIN_PATH) {
            const auto utf8Path = entry.bundlePath().u8string();
            path.assign(utf8Path.begin(), utf8Path.end());
        }
        const std::string* s = field == UH_PLUGIN_PATH ? &path : pluginField(entry, field);
        if (!s)
            return -1;
        if (buf && buf_len > 0) {
            const size_t n = (std::min)(s->size(), static_cast<size_t>(buf_len - 1));
            std::memcpy(buf, s->data(), n);
            buf[n] = '\0';
        }
        return static_cast<int32_t>(s->size());
    } catch (...) {
        return -1;
    }
}

int32_t uh_restore_plugin(UhHost* host, const char* format, const char* id,
                          const char* name, const char* vendor, const char* path) {
    if (!host || host->scanning.load() || !format || !id || !name || !vendor || !path)
        return -1;
    try {
        auto bundle = std::filesystem::u8path(path);
        if (bundle.empty() || !std::filesystem::exists(bundle))
            return -1;
        AudioPluginCatalogEntry entry;
        entry.format(format);
        entry.pluginId(id);
        entry.displayName(name);
        entry.vendorName(vendor);
        entry.bundlePath(bundle);
        host->catalog.push_back(std::move(entry));
        return static_cast<int32_t>(host->catalog.size() - 1);
    } catch (...) {
        return -1;
    }
}

void uh_instance_create(UhHost* host, int32_t index, uint32_t sample_rate,
                        uint32_t buffer_size, UhInstanceFn done, void* user) {
    // The result is always delivered from uh_pump(), also when uapmd reports it at once.
    auto report = [done, user](int32_t instanceId, std::string error) {
        g_loop->post([done, user, instanceId, error = std::move(error)] {
            if (done)
                done(user, instanceId, error.empty() ? nullptr : error.c_str());
        });
    };
    if (!host || !g_loop)
        return;
    try {
        if (index < 0 || static_cast<size_t>(index) >= host->catalog.size()) {
            report(-1, "plugin index out of range");
            return;
        }
        auto instance = std::make_unique<OwnedInstance>(
            host->catalog[static_cast<size_t>(index)], sample_rate, buffer_size);
        const auto instanceId = host->next_instance_id++;
        auto* lifecycle = instance->lifecycle.get();
        auto reported = std::make_shared<std::atomic<bool>>(false);
        auto complete = [host, done, user, instanceId, reported](std::string error) {
            if (reported->exchange(true))
                return;
            g_loop->post([host, done, user, instanceId, error = std::move(error)] {
                --host->pending_instances;
                if (!error.empty())
                    destroyInstance(host, instanceId);
                if (done)
                    done(user, error.empty() ? instanceId : -1,
                         error.empty() ? nullptr : error.c_str());
            });
        };
        host->instances.emplace(instanceId, std::move(instance));
        ++host->pending_instances;
        try {
            lifecycle->makeAlive(complete);
        } catch (const std::exception& e) {
            complete(e.what()[0] ? e.what() : "plugin instantiation failed");
        } catch (...) {
            complete("plugin instantiation failed");
        }
    } catch (const std::exception& e) {
        report(-1, e.what());
    } catch (...) {
        report(-1, "plugin instantiation failed");
    }
}

void uh_instance_destroy(UhHost* host, int32_t instance_id) {
    if (!host)
        return;
    try {
        destroyInstance(host, instance_id);
    } catch (...) {
    }
}

UhProcessor* uh_processor_create(UhHost* host, int32_t instance_id,
                                 uint32_t sample_rate, uint32_t max_frames) {
    if (!host || max_frames == 0)
        return nullptr;
    try {
        auto* instance = getInstance(host, instance_id);
        if (!instance)
            return nullptr;
        auto* buses = instance->audioBuses();
        if (!buses)
            return nullptr;

        auto processor = std::make_unique<UhProcessor>(kEventBufferBytes);
        processor->instance = instance;
        processor->max_frames = max_frames;
        processor->master.sampleRate(static_cast<int32_t>(sample_rate));
        processor->master.isPlaying(true);

        // The context gets the plugin's own bus layout, with the main buses first
        // (the same arrangement uapmd's scan verification uses).
        const auto& inputBuses = buses->audioInputBuses();
        const auto& outputBuses = buses->audioOutputBuses();
        const size_t numIn = inputBuses.size();
        const size_t numOut = outputBuses.size();
        int32_t mainIn = buses->mainInputBusIndex();
        int32_t mainOut = buses->mainOutputBusIndex();
        if (mainIn < 0 && numIn > 0)
            mainIn = 0;
        if (mainOut < 0 && numOut > 0)
            mainOut = 0;
        const auto mainInChannels = (mainIn >= 0 && static_cast<size_t>(mainIn) < numIn)
            ? inputBuses[static_cast<size_t>(mainIn)]->channelLayout().channels() : 0;
        const auto mainOutChannels = (mainOut >= 0 && static_cast<size_t>(mainOut) < numOut)
            ? outputBuses[static_cast<size_t>(mainOut)]->channelLayout().channels() : 0;

        auto& ctx = processor->ctx;
        ctx.configureMainBus(static_cast<int32_t>(mainInChannels),
                             static_cast<int32_t>(mainOutChannels), max_frames);
        for (size_t i = 0; i < numIn; ++i)
            if (static_cast<int32_t>(i) != mainIn)
                ctx.addAudioIn(static_cast<int32_t>(inputBuses[i]->channelLayout().channels()), max_frames);
        for (size_t i = 0; i < numOut; ++i)
            if (static_cast<int32_t>(i) != mainOut)
                ctx.addAudioOut(static_cast<int32_t>(outputBuses[i]->channelLayout().channels()), max_frames);

        return processor.release();
    } catch (...) {
        return nullptr;
    }
}

void uh_processor_destroy(UhProcessor* processor) {
    delete processor;
}

int32_t uh_processor_process(UhProcessor* processor,
                             const uint32_t* ump_words, int32_t ump_word_count,
                             float* out_interleaved, int32_t out_channels, int32_t frames) {
    if (!out_interleaved || out_channels <= 0 || frames <= 0)
        return -1;
    const size_t total = static_cast<size_t>(frames) * static_cast<size_t>(out_channels);
    std::memset(out_interleaved, 0, total * sizeof(float));
    if (!processor || static_cast<uint32_t>(frames) > processor->max_frames)
        return -1;

    try {
        // Tells remidy (and through it the plugin's thread checks) that this thread
        // is the audio thread, as uapmd's own engine does around its audio callback.
        remidy::AudioThreadScope audioThreadRole;
        auto& ctx = processor->ctx;
        ctx.frameCount(frames);
        // These also reset the event positions, so they come before the events.
        ctx.clearAudioInputs();
        ctx.clearAudioOutputs();

        auto& eventIn = ctx.eventIn();
        size_t eventBytes = 0;
        if (ump_words && ump_word_count > 0) {
            eventBytes = static_cast<size_t>(ump_word_count) * sizeof(uint32_t);
            if (eventBytes > eventIn.maxMessagesInBytes())
                eventBytes = eventIn.maxMessagesInBytes() / sizeof(uint32_t) * sizeof(uint32_t);
            std::memcpy(eventIn.getMessages(), ump_words, eventBytes);
        }
        eventIn.position(eventBytes);

        const auto status = processor->instance->processAudio(ctx);
        eventIn.position(0);
        processor->master.playbackPositionSamples(processor->master.playbackPositionSamples() + frames);
        if (status != 0)
            return status;

        if (ctx.audioOutBusCount() < 1)
            return 0;
        const int32_t busChannels = ctx.outputChannelCount(0);
        for (int32_t ch = 0; ch < out_channels; ++ch) {
            // Mono plugins are copied to both of the first two device channels.
            const int32_t srcCh = ch < busChannels ? ch : ((busChannels == 1 && ch < 2) ? 0 : -1);
            if (srcCh < 0)
                continue;
            const float* src = ctx.getFloatOutBuffer(0, static_cast<uint32_t>(srcCh));
            if (!src)
                continue;
            float* dst = out_interleaved + ch;
            for (int32_t i = 0; i < frames; ++i)
                dst[static_cast<size_t>(i) * static_cast<size_t>(out_channels)] = src[i];
        }
        return 0;
    } catch (...) {
        std::memset(out_interleaved, 0, total * sizeof(float));
        return -5;
    }
}

int32_t uh_ui_show(UhHost* host, int32_t instance_id) {
    if (!host)
        return UH_ERR_NO_INSTANCE;
    try {
        auto* instance = getInstance(host, instance_id);
        if (!instance)
            return UH_ERR_NO_INSTANCE;
        if (!instance->hasUISupport())
            return UH_ERR_NO_UI;

        auto& window = host->windows[instance_id];
        if (!window) {
            const std::string title = instance->displayName() + " (" + instance->formatName() + ")";
            window = remidy::gui::ContainerWindow::create(title.c_str(), 800, 600, [host, instance_id] {
                // The close button: the container hides itself, the editor is kept.
                if (auto* i = getInstance(host, instance_id))
                    i->hideUI();
            });
            if (!window) {
                host->windows.erase(instance_id);
                return UH_ERR_CREATE_UI;
            }
            // Resizing by the user is left out of this first step.
            window->setResizable(false);
        }
        auto* container = window.get();
        container->show(true);

        if (!host->ui_created.contains(instance_id)) {
            auto onPluginResize = [container](uint32_t width, uint32_t height) {
                container->resize(static_cast<int>(width), static_cast<int>(height));
                return true;
            };
            if (!instance->createUI(false, container->getHandle(), onPluginResize)) {
                container->show(false);
                return UH_ERR_CREATE_UI;
            }
            host->ui_created.insert(instance_id);
            uint32_t width = 0, height = 0;
            if (instance->getUISize(width, height) && width > 0 && height > 0)
                container->resize(static_cast<int>(width), static_cast<int>(height));
        }
        if (!instance->showUI())
            return UH_ERR_SHOW_UI;
        return UH_OK;
    } catch (...) {
        return UH_ERR_EXCEPTION;
    }
}

void uh_ui_hide(UhHost* host, int32_t instance_id) {
    if (!host)
        return;
    try {
        if (auto* instance = getInstance(host, instance_id))
            instance->hideUI();
        auto it = host->windows.find(instance_id);
        if (it != host->windows.end() && it->second)
            it->second->show(false);
    } catch (...) {
    }
}

} // extern "C"
