// Exercises the actual compiled remidy backend and queued completion count.
#include <remidy/remidy.hpp>
#include <chrono>
#include <deque>
#include <iostream>
#include <mutex>
#include <stdexcept>
#include <thread>

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
            {
                std::lock_guard lock(mutex);
                if (tasks.empty()) break;
                fn = std::move(tasks.front()); tasks.pop_front();
            }
            fn();
        }
    }
};
static void require(bool ok, const char* why) { if (!ok) throw std::runtime_error(why); }
int main(int argc, char** argv) {
    try {
        require(argc == 2, "usage: backend_test test_plugin.clap");
        Loop loop;
        remidy::setEventLoop(&loop);
        remidy::EventLoop::initializeOnUIThread();
        std::vector<std::string> paths;
        auto format = remidy::PluginFormatCLAP::create(paths);
        for (const auto* id : {"cat.test.state.normal", "cat.test.state.context", "cat.test.state.none"}) {
            remidy::PluginCatalogEntry entry;
            entry.format() = "CLAP"; entry.pluginId() = id;
            entry.bundlePath() = std::filesystem::absolute(argv[1]);
            std::unique_ptr<remidy::PluginInstance> instance;
            bool created = false;
            format->createInstance(&entry, {remidy::PluginUIThreadRequirement::AllNonAudioOperation}, [&](auto value, auto error) {
                require(error.empty(), "instance error"); instance = std::move(value); created = true;
            });
            auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(20);
            while (!created) {
                require(std::chrono::steady_clock::now() < deadline, "instance timeout");
                remidy::EventLoop::processQueuedTasks();
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
            require(bool(instance), "missing instance");
            for (uint8_t value : {1, 0, 2, 3, 1}) {
                int count = 0;
                std::string error;
                instance->states()->loadState({value}, remidy::PluginStateSupport::StateContextType::Project,
                    true, nullptr, [&](std::string result, void*) { ++count; error = std::move(result); });
                deadline = std::chrono::steady_clock::now() + std::chrono::seconds(20);
                while (!count) {
                    require(std::chrono::steady_clock::now() < deadline, "state timeout");
                    remidy::EventLoop::processQueuedTasks();
                }
                remidy::EventLoop::processQueuedTasks();
                require(count == 1, "completion was not exactly once");
                bool success = value == 1 && std::string(id) != "cat.test.state.none";
                require(error.empty() == success, "incorrect success/error propagation");
                std::cout << id << " input=" << int(value) << " callbacks=" << count << " error=" << error << std::endl;
            }
        }
        remidy::setEventLoop(nullptr);
    } catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
