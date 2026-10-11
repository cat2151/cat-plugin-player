# Draft: CLAP UMP notes ignore input note-port dialects

Status: **未報告**; no external message has been sent.

Affected upstream revisions: `bc39b50faf2f2b02668de82d3d95f2bdbafd3c34` and
`a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe` (CLAP sources unchanged between them).

`PluginInstanceCLAP.Events.cpp` unconditionally turns UMP Note On/Off into
`CLAP_EVENT_NOTE_ON/OFF`. sforzando CLAP 2.1.2.4 input port 0 advertises
supported/preferred dialect 2 (`CLAP_NOTE_DIALECT_MIDI`) and does not advertise
native CLAP Note. Both MIDI1 and MIDI2 UMP enter the same typed dispatcher, so
changing the UMP input type alone does not avoid the mismatch. `onCC` also drops
the application-used modulation and all-sound-off messages.

Suggested correction: query and cache input note-port capability on the main
thread while deactivated, then emit native CLAP when supported or MIDI otherwise.
Keep On/Off port, channel, key and sample time consistent; avoid double emission
and accidental zero velocity when reducing MIDI2 Note On to MIDI1. CC1/CC120 need
MIDI on MIDI-capable ports. This application's local patch deliberately avoids
general MIDI2/MPE/CC expansion.

The local generated-source repair preserves the upstream checkout. It includes
tests for native/MIDI/both/unsupported ports, one emission, On/Off fields, velocity,
time, current CC, nonmutating generation/reapply and stopping on upstream drift.
The independent audio reproduction restores identical saved TableWarp2 state into
fresh instances at 48 kHz / 256 frames, initializes CC1 identically, sends note
60/velocity 100/channel 0 at equal times, and records RMS/peak/process status.
Separate off and held arms test Note Off against natural decay/release.

Measured 2026-10-07: same 421-byte TableWarp2 state SHA256
`4931169322f524938bb7fdd6352b3835eb7c284e16517078c61514076d3938c2`.
Direct native Note sounding RMS/peak were 1e-20/1e-20 (silence floor); direct MIDI
RMS/peak were 0.0758554/0.272482. Every one of 1500 direct process calls returned
CONTINUE. The repaired shim's UMP path produced the same MIDI audio measurements
and 1500 successful process calls. With Note Off, early release RMS was 0.00666599
and late release reached 1e-20; the held control had early RMS 0.0180376 and late
RMS 4.84215e-6. Thus the difference is not merely the sample's natural decay.
Surge XT 1.3.4 (supported 7/preferred native 1) remained audible through the repaired
native branch, sounding RMS/peak 0.0800660/0.212675 and silence after release.
Product config/favorite/state hashes were unchanged. Local policy/application
tests and ordinary cached release build passed; a complete new-environment build
was not exercised. Human GUI/listening acceptance and VST3 remain unconfirmed.
