#pragma once
// The shim's JR Timestamps count samples (uapmd_shim.h); remidy reads them as the UMP
// definition's 1/31250 s ticks and converts back with tick * rate / 31250.
#include <cstddef>
#include <cstdint>

namespace uh {

inline size_t umpWords(uint32_t word) {
    switch (word >> 28) {
    case 0x3: case 0x4: case 0x8: case 0x9: case 0xA: return 2;
    case 0xB: case 0xC: return 3;
    case 0x5: case 0xD: case 0xE: case 0xF: return 4;
    default: return 1;
    }
}

// In place. Each event's sample position from the block start maps to the last tick
// remidy converts to that sample or earlier, so no event moves past its sample.
inline void jrSamplesToTicks(uint32_t* words, size_t count, uint32_t sampleRate) {
    if (sampleRate == 0)
        return;
    uint64_t samples = 0;
    uint64_t ticks = 0;
    for (size_t i = 0; i < count; i += umpWords(words[i])) {
        const uint32_t word = words[i];
        if ((word >> 28) != 0 || ((word >> 20) & 0xF) != 2)
            continue;
        samples += word & 0xFFFF;
        uint64_t target = (samples * 31250 + sampleRate - 1) / sampleRate;
        if (target * sampleRate / 31250 > samples)
            --target;
        // The 16-bit field holds about 2 s of ticks at any rate; no block is that long.
        words[i] = (word & 0xFFFF0000u) | static_cast<uint32_t>((target - ticks) & 0xFFFF);
        ticks = target;
    }
}

} // namespace uh
