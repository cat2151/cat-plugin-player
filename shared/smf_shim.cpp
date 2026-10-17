#include <umppi/umppi.hpp>
#include <algorithm>
#include <cstring>
#include <limits>
#include <sstream>
#include <stdexcept>

namespace {
// Validate lengths/statuses before handing the stream to umppi. No event model
// is produced here; umppi remains the sole source of parsed events.
void validate(const uint8_t* data, size_t size) {
    size_t p = 0;
    auto require = [&](bool condition) {
        if (!condition)
            throw std::runtime_error("Invalid or truncated SMF");
    };
    auto word = [&](size_t n) -> uint32_t {
        require(n <= size - p);
        uint32_t result = 0;
        while (n--)
            result = (result << 8) | data[p++];
        return result;
    };
    require(size >= 14);
    require(word(4) == 0x4d546864 && word(4) == 6);
    const auto format = word(2), tracks = word(2), division = word(2);
    require(format <= 2 && tracks > 0 && tracks <= 32767);
    require(format != 0 || tracks == 1);
    if (division & 0x8000) {
        auto fps = static_cast<int8_t>(division >> 8);
        require((fps == -24 || fps == -25 || fps == -29 || fps == -30) && (division & 255));
    } else
        require(division > 0);
    for (uint32_t t = 0; t < tracks; ++t) {
        require(word(4) == 0x4d54726b);
        auto length = word(4);
        require(length <= INT32_MAX && length <= size - p);
        const size_t end = p + length;
        auto byte = [&]() {
            require(p < end);
            return data[p++];
        };
        auto vlq = [&]() -> uint32_t {
            uint32_t v = 0;
            for (int i = 0; i < 4; ++i) {
                auto b = byte();
                v = (v << 7) | (b & 127);
                if (!(b & 128))
                    return v;
            }
            throw std::runtime_error("Invalid SMF VLQ");
        };
        uint8_t running = 0;
        while (p < end) {
            vlq();
            require(p < end);
            uint8_t status = data[p];
            if (status & 128) {
                ++p;
                if (status < 0xf0)
                    running = status;
                else
                    running = 0;
            } else {
                status = running;
                require(status >= 0x80 && status < 0xf0);
            }
            if (status == 0xff || status == 0xf0 || status == 0xf7) {
                if (status == 0xff)
                    byte();
                auto n = vlq();
                require(n <= end - p);
                p += n;
            } else {
                require(status < 0xf0);
                auto n = umppi::Midi1Message::fixedDataSize(status);
                while (n--)
                    require(byte() < 128);
            }
        }
    }
    require(p == size);
}

struct Event {
    uint32_t delta;
    uint8_t status, meta_type;
    std::vector<uint8_t> data;
};
struct Music {
    uint8_t format;
    uint16_t division;
    std::vector<std::vector<Event>> tracks;
};
}

extern "C" {
struct RawEvent {
    uint32_t delta;
    uint8_t status, meta_type;
    const uint8_t* data;
    size_t size;
};
void* smf_read(const uint8_t* data, size_t size, char* error, size_t error_size) noexcept {
    try {
        validate(data, size);
        std::istringstream stream(std::string(reinterpret_cast<const char*>(data), size), std::ios::binary);
        auto source = umppi::Midi1Reader(stream).read();
        auto result = std::make_unique<Music>();
        result->format = source.format;
        result->division = static_cast<uint16_t>(source.deltaTimeSpec);
        for (const auto& track : source.tracks) {
            std::vector<Event> events;
            for (const auto& event : track.events) {
                auto& m = *event.message;
                Event out{static_cast<uint32_t>(event.deltaTime), m.getStatusByte(), 0, {}};
                if (auto compound = dynamic_cast<const umppi::Midi1CompoundMessage*>(&m)) {
                    out.meta_type = m.getMetaType();
                    const auto& payload = compound->getExtraData();
                    const auto offset = compound->getExtraDataOffset(), length = compound->getExtraDataLength();
                    if (offset > payload.size() || length > payload.size() - offset)
                        throw std::runtime_error("umppi payload bounds mismatch");
                    out.data.assign(payload.begin() + offset, payload.begin() + offset + length);
                } else {
                    out.data.push_back(m.getMsb());
                    if (umppi::Midi1Message::fixedDataSize(out.status) == 2)
                        out.data.push_back(m.getLsb());
                }
                events.push_back(std::move(out));
            }
            result->tracks.push_back(std::move(events));
        }
        return result.release();
    } catch (const std::exception& e) {
        if (error_size) {
            const size_t length = std::min(error_size - 1, std::strlen(e.what()));
            std::memcpy(error, e.what(), length);
            error[length] = 0;
        }
    } catch (...) {
        if (error_size) {
            const char* message = "Unknown C++ exception";
            const auto length = std::min(error_size - 1, std::strlen(message));
            std::memcpy(error, message, length);
            error[length] = 0;
        }
    }
    return nullptr;
}
void smf_free(void* m) noexcept { delete static_cast<Music*>(m); }
uint8_t smf_format(const Music* m) noexcept { return m->format; }
uint16_t smf_division(const Music* m) noexcept { return m->division; }
size_t smf_tracks(const Music* m) noexcept { return m->tracks.size(); }
size_t smf_events(const Music* m, size_t t) noexcept { return m->tracks[t].size(); }
void smf_event(const Music* m, size_t t, size_t i, RawEvent* out) noexcept {
    const auto& e = m->tracks[t][i];
    *out = {e.delta, e.status, e.meta_type, e.data.data(), e.data.size()};
}
}
