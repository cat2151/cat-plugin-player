#include "note_dialect.h"
#include <cstdlib>
#include <iostream>
#include <cstring>
#define CHECK(x) do { if (!(x)) { std::cerr << #x << ':' << __LINE__ << '\n'; std::exit(1); } } while (0)
int main() {
    alignas(8) unsigned char bytes[128]{};
    int count = 0;
    auto allocate = [&](size_t size) -> void* { CHECK(size <= sizeof(bytes)); ++count; return bytes; };
    for (uint32_t dialect : {1u, 2u, 3u, 0u, 4u}) {
        for (bool on : {true, false}) {
            count = 0;
            uh::clap_dialect::note(allocate, dialect, on, 3, 7, 60, 100 << 9, 41);
            CHECK(count == ((dialect & 3) ? 1 : 0));
            if (!count) continue;
            auto* header = reinterpret_cast<clap_event_header_t*>(bytes);
            CHECK(header->time == 41 && header->space_id == CLAP_CORE_EVENT_SPACE_ID);
            if (dialect & 1) {
                auto* event = reinterpret_cast<clap_event_note_t*>(bytes);
                CHECK(header->type == (on ? CLAP_EVENT_NOTE_ON : CLAP_EVENT_NOTE_OFF));
                CHECK(event->port_index == 3 && event->channel == 7 && event->key == 60);
                CHECK(event->note_id == -1 && event->velocity > 0);
            } else {
                auto* event = reinterpret_cast<clap_event_midi_t*>(bytes);
                CHECK(header->type == CLAP_EVENT_MIDI && event->port_index == 3);
                CHECK(event->data[0] == (on ? 0x97 : 0x87));
                CHECK(event->data[1] == 60 && event->data[2] == 100);
            }
        }
    }
    uh::clap_dialect::note(allocate, 2, true, 0, 0, 60, 1, 0);
    CHECK(reinterpret_cast<clap_event_midi_t*>(bytes)->data[2] == 1);
    for (uint32_t dialect : {1u, 2u, 3u}) {
        for (uint8_t index : {1, 120, 7}) {
            count = 0;
            uh::clap_dialect::cc(allocate, dialect, 2, 4, index, 127u << 25, 9);
            CHECK(count == ((dialect & 2) && index != 7 ? 1 : 0));
            if (count) {
                auto* event = reinterpret_cast<clap_event_midi_t*>(bytes);
                CHECK(event->header.time == 9 && event->port_index == 2);
                CHECK(event->data[0] == 0xB4 && event->data[1] == index && event->data[2] == 127);
            }
        }
    }
    uh::clap_dialect::note([](size_t) -> void* { return nullptr; }, 2, true, 0, 0, 60, 1, 0);
    std::cout << "dialect policy: single event, On/Off, velocity, port/time, CC1/120 PASS\n";
}
