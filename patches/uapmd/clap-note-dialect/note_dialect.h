#pragma once
#include <clap/events.h>
#include <clap/ext/note-ports.h>
#include <cstdint>

namespace uh::clap_dialect {
// Zero means an explicitly unsupported port. Missing extension is handled by
// the caller's legacy CLAP fallback, not by guessing MIDI here.
template<class Allocate>
void note(Allocate allocate, uint32_t dialects, bool on, uint16_t port,
          uint8_t channel, uint8_t key, uint16_t velocity, uint32_t time) {
    if (dialects & CLAP_NOTE_DIALECT_CLAP) {
        auto* event = static_cast<clap_event_note_t*>(allocate(sizeof(clap_event_note_t)));
        if (!event) return;
        *event = {};
        event->header = {sizeof(*event), time, CLAP_CORE_EVENT_SPACE_ID,
                         static_cast<uint16_t>(on ? CLAP_EVENT_NOTE_ON : CLAP_EVENT_NOTE_OFF), 0};
        event->note_id = -1;
        event->port_index = static_cast<int16_t>(port);
        event->channel = channel;
        event->key = key;
        event->velocity = static_cast<double>(velocity) / UINT16_MAX;
    } else if (dialects & CLAP_NOTE_DIALECT_MIDI) {
        auto* event = static_cast<clap_event_midi_t*>(allocate(sizeof(clap_event_midi_t)));
        if (!event) return;
        *event = {};
        event->header = {sizeof(*event), time, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_MIDI, 0};
        event->port_index = port;
        const auto v = static_cast<uint8_t>(velocity >> 9);
        event->data[0] = static_cast<uint8_t>((on ? 0x90 : 0x80) | channel);
        event->data[1] = key;
        event->data[2] = on && velocity && !v ? 1 : v;
    }
}

template<class Allocate>
void cc(Allocate allocate, uint32_t dialects, uint16_t port, uint8_t channel,
        uint8_t index, uint32_t value, uint32_t time) {
    // This application uses modulation and all-sound-off only.
    if (!(dialects & CLAP_NOTE_DIALECT_MIDI) || (index != 1 && index != 120)) return;
    auto* event = static_cast<clap_event_midi_t*>(allocate(sizeof(clap_event_midi_t)));
    if (!event) return;
    *event = {};
    event->header = {sizeof(*event), time, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_MIDI, 0};
    event->port_index = port;
    event->data[0] = static_cast<uint8_t>(0xB0 | channel);
    event->data[1] = index;
    event->data[2] = static_cast<uint8_t>(value >> 25);
}
}
