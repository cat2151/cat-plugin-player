// Compile with MSVC /std:c++20 /EHsc /Ishim.
#include "jr_timestamp.h"
#include <cassert>
#include <cstdint>
#include <initializer_list>

// What remidy does with the converted words: sum ticks, then tick * rate / 31250.
static uint64_t remidySample(const uint32_t* words, size_t count, uint32_t rate) {
    uint64_t ticks = 0;
    for (size_t i = 0; i < count; i += uh::umpWords(words[i]))
        if ((words[i] >> 28) == 0 && ((words[i] >> 20) & 0xF) == 2)
            ticks += words[i] & 0xFFFF;
    return ticks * rate / 31250;
}

int main() {
    for (uint32_t rate : {44100u, 48000u, 96000u}) {
        for (uint32_t at = 0; at < 1024; ++at) {
            // A two-word message before the timestamp must not be read as one.
            uint32_t words[] = {0x40900000u, 0x00200000u, 0x00200000u | at, 0x20903C64u};
            uh::jrSamplesToTicks(words, 4, rate);
            assert(words[0] == 0x40900000u && words[1] == 0x00200000u);
            const uint64_t sample = remidySample(words, 3, rate);
            // Never past the requested sample, early by less than one tick.
            assert(sample <= at && (at - sample) * 31250 < rate);
        }
        // Positions accumulate: each event keeps its own sample, not drifting.
        uint32_t words[] = {0x00200000u | 300, 0x20903C64u, 0x00200000u | 400, 0x20803C00u};
        uh::jrSamplesToTicks(words, 4, rate);
        const uint64_t first = remidySample(words, 1, rate);
        const uint64_t second = remidySample(words, 4, rate);
        assert(first <= 300 && (300 - first) * 31250 < rate);
        assert(second <= 700 && (700 - second) * 31250 < rate);
    }
    // Rate 0 leaves the words as given.
    uint32_t words[] = {0x00200000u | 5};
    uh::jrSamplesToTicks(words, 1, 0);
    assert(words[0] == (0x00200000u | 5));
}
