// No device, plugin scan, registration or user settings. Exercise compiled remidy.
#include <remidy/remidy.hpp>
#include <chrono>
#include <deque>
#include <iostream>
#include <mutex>
#include <stdexcept>
#include <thread>
#include <cmath>
#include "backend_api.h"
#include "fixed_changes.h"
#include <cstdlib>
#include <new>

static thread_local bool trackAllocations{};
static thread_local size_t allocations{};
void* operator new(size_t size) {
    if (trackAllocations) ++allocations;
    if (auto* p = std::malloc(size ? size : 1)) return p;
    throw std::bad_alloc();
}
void* operator new[](size_t size) { return ::operator new(size); }
void operator delete(void* p) noexcept { std::free(p); }
void operator delete[](void* p) noexcept { std::free(p); }
void operator delete(void* p, size_t) noexcept { std::free(p); }
void operator delete[](void* p, size_t) noexcept { std::free(p); }

class Loop : public remidy::EventLoop {
    std::deque<std::function<void()>> tasks;
    std::mutex mutex;
    std::thread::id main = std::this_thread::get_id();
    void initializeOnUIThreadImpl() override {}
    bool runningOnMainThreadImpl() override { return main == std::this_thread::get_id(); }
    void enqueueTaskOnMainThreadImpl(std::function<void()>&& fn) override {
        std::lock_guard lock(mutex); tasks.push_back(std::move(fn));
    }
    void startImpl() override {}
    void stopImpl() override {}
    void processQueuedTasksImpl() override {
        for (;;) {
            std::function<void()> fn;
            { std::lock_guard lock(mutex); if (tasks.empty()) break;
                fn = std::move(tasks.front()); tasks.pop_front(); }
            fn();
        }
    }
};
static void require(bool ok, const char* why) { if (!ok) throw std::runtime_error(why); }
static std::unique_ptr<remidy::PluginInstance> create(remidy::PluginFormat& format, remidy::PluginCatalogEntry& entry) {
    std::unique_ptr<remidy::PluginInstance> instance;
    bool done = false;
    format.createInstance(&entry, {remidy::PluginUIThreadRequirement::AllNonAudioOperation}, [&](auto value, auto error) {
        require(error.empty(), error.c_str()); instance = std::move(value); done = true;
    });
    auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(20);
    while (!done) {
        require(std::chrono::steady_clock::now() < deadline, "instance timeout");
        remidy::EventLoop::processQueuedTasks(); std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    require(bool(instance), "missing instance");
    remidy::PluginInstance::ConfigurationRequest config;
    config.sampleRate = 48000; config.bufferSizeInSamples = 64;
    require(instance->configure(config) == remidy::StatusCode::OK, "configure failed");
    return instance;
}
static void diagnostic(remidy::PluginFormat& format, const char* path, const char* id, bool expectBroken) {
    remidy::PluginCatalogEntry entry;
    entry.format() = format.name(); entry.pluginId() = id;
    entry.bundlePath() = std::filesystem::absolute(path);
    auto instance = create(format, entry);
    auto untouched = create(format, entry);
    auto* params = instance->parameters();
    if (!expectBroken) require(format.name() == "CLAP" ? uh::automation::clapPrepare(params) : uh::automation::vst3Prepare(params), "prepare failed");
    uh::automation::Target targets[2];
    if (!expectBroken) {
        auto resolve = format.name() == "CLAP" ? uh::automation::clapResolve : uh::automation::vst3Resolve;
        require(resolve(params, 0, &targets[0]) && resolve(params, 1, &targets[1]), "resolve failed");
    }
    auto& metadata = params->parameters();
    require(metadata.size() >= 4, "metadata missing");
    require(metadata[0]->stableId() == "101" && metadata[1]->stableId() == "118", "stable ID lost");
    bool flagsCorrect = metadata[0]->automatable() && !metadata[2]->automatable();
    if (!expectBroken) require(flagsCorrect, "incorrect automation flags");
    auto display = format.name() == "VST3" && !expectBroken ? uh::automation::vst3Text(params, 0, 0.7) : params->valueToString(0, 0.7);
    std::cout << format.name() << " flags_correct=" << flagsCorrect << " display=" << display << '\n';
    remidy::MasterContext master;
    master.sampleRate(48000); master.isPlaying(true);
    remidy::AudioProcessContext ctx(master, 4096);
    ctx.configureMainBus(0, 2, 64); ctx.frameCount(64);
    instance->startProcessing();
    {
        remidy::AudioThreadScope audio;
        require(instance->process(ctx) == remidy::StatusCode::OK, "warmup failed");
    }
    allocations = 0;
    {
        remidy::AudioThreadScope audio;
        trackAllocations = !expectBroken;
        if (expectBroken) {
            require(params->enqueueParameterRT(0, 0.7, 0) == remidy::StatusCode::OK, "enqueue first failed");
            require(params->enqueueParameterRT(1, 0.8, 0) == remidy::StatusCode::OK, "enqueue second failed");
        } else {
            auto enqueue = format.name() == "CLAP" ? uh::automation::clapApply : uh::automation::vst3Apply;
            require(enqueue(params, targets[0], 0.7) == 0 && enqueue(params, targets[1], 0.8) == 0, "automation enqueue failed");
        }
        auto* words = static_cast<uint32_t*>(ctx.eventIn().getMessages());
        words[0] = 0x40903c00; words[1] = 0xffff0000; ctx.eventIn().position(8);
        require(instance->process(ctx) == remidy::StatusCode::OK, "process failed");
        trackAllocations = false;
    }
    if (!expectBroken) require(allocations == 0, "host allocated during parameter/note process");
    if (!expectBroken) {
        auto apply=format.name()=="CLAP" ? uh::automation::clapApply : uh::automation::vst3Apply;
        auto note=format.name()=="CLAP" ? uh::automation::clapNote : uh::automation::vst3Note;
        ctx.eventIn().position(0);allocations=0;
        { remidy::AudioThreadScope audio;trackAllocations=true;
            require(apply(params,targets[0],0.2)==0 && apply(params,targets[1],0.3)==0,"ordered baseline failed");
            require(note(params,0x20903d64)==0,"ordered early note failed");
            require(apply(params,targets[0],0.7)==0 && apply(params,targets[1],0.8)==0,"ordered later parameter failed");
            require(note(params,0x20903c64)==0,"ordered later note failed");
            require(instance->process(ctx)==remidy::StatusCode::OK,"ordered process failed");
            trackAllocations=false;
        }
        require(allocations==0,"host allocated during ordered collision process");
    }
    float left = ctx.getFloatOutBuffer(0, 0)[0], right = ctx.getFloatOutBuffer(0, 1)[0];
    bool delivered = std::abs(left - 0.7f) < 1e-6 && std::abs(right - 0.8f) < 1e-6;
    double other = 0;
    untouched->parameters()->parameters();
    bool otherRead = format.name() == "VST3" && !expectBroken
        ? uh::automation::vst3Current(untouched->parameters(), 0, &other) == 0
        : untouched->parameters()->getParameter(0, &other) == remidy::StatusCode::OK;
    double expectedOther = format.name() == "VST3" && expectBroken ? 136.0 : 0.2;
    require(otherRead && std::abs(other - expectedOther) < 1e-6,
        "other instance was modified");
    instance->stopProcessing();
    std::cout << format.name() << " processor=" << left << ',' << right << " other=" << other << '\n';
    require(expectBroken ? !delivered : delivered, expectBroken ? "expected baseline defect absent" : "incorrect processor values/order");
    // Destroy and recreate: IDs resolve from the new instance, never a saved index/pointer.
    instance.reset(); instance = create(format, entry);
    require(instance->parameters()->parameters()[1]->stableId() == "118", "recreated identity mismatch");
}
int main(int argc, char** argv) {
    try {
        require(argc == 4, "usage: backend_test fixture.clap fixture.vst3 baseline|fixed");
        Loop loop; remidy::setEventLoop(&loop); remidy::EventLoop::initializeOnUIThread();
        std::vector<std::string> paths;
        auto clap = remidy::PluginFormatCLAP::create(paths);
        auto vst3 = remidy::PluginFormatVST3::create(paths);
        bool broken = std::string(argv[3]) == "baseline";
        if (!broken) {
            auto changes = std::make_unique<uh::automation::FixedChanges>();
            Steinberg::int32 index;
            for (uint32_t id = 0; id < 256; ++id) require(changes->addParameterData(id, index), "queue capacity missing");
            require(!changes->addParameterData(256, index), "queue overflow accepted");
            auto* q = changes->getParameterData(0);
            for (int i = 0; i < 64; ++i) require(q->addPoint(0, 0.5, index) == Steinberg::kResultOk, "point capacity missing");
            require(q->addPoint(0, 0.5, index) != Steinberg::kResultOk, "point overflow accepted");
            changes->clear(); require(changes->getParameterCount() == 0, "queue clear failed");
        }
        diagnostic(*clap, argv[1], "cat.test.parameters", broken);
        diagnostic(*vst3, argv[2], "132132131234123456785678ABCDEF01", broken);
        remidy::setEventLoop(nullptr);
    } catch (const std::exception& error) { trackAllocations = false; std::cerr << error.what() << '\n'; return 1; }
}
