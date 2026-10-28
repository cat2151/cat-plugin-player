# Direct global parameter automation: CLAP metadata/addressing and VST3 RT queue

Unsent report draft. Investigated revision: `a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe`.

CLAP `populateParameterList` passes `true` for PluginParameter's automatable
argument and the automatable flag for readable. Read-only parameters consequently
appear writable. `enqueueParameterRT` zero-initializes the event, leaves note_id/key
zero, sets port/channel zero and sets LIVE even for scheduled global automation.
Global addressing should use -1 for note_id/key/port/channel and scheduled input
should not be marked live.

VST3 `enqueueParameterRT` keeps the last nonmatching queue when the requested ID
is absent. Updating ID A followed by ID B can put B's point into A's queue. It
also calls `plainParamToNormalized` on the controller, invokes mutex-backed change
listeners, and relies on queue allocation/deallocation in the processing path.
An independent normalized, bounded input path avoids those RT operations without
changing the meaning of the legacy plain-value API.

The local fixtures expose two IDs (101 and 118) and render processor-observed
values into stereo buffers. Both baseline paths return invalid results for a
two-ID update followed by Note On. The repaired path returns 0.7/0.8 while an
untouched instance remains 0.2/0.3. VST3's fixture has a nonlinear plain range
distinct from normalized storage. Metadata, stale bindings, bounded capacity,
rescan invalidation and existing note/state behavior have separate regression
tests. No audio device or plugin catalog scan is needed.

VST3 current controller values also lag behind processor-applied automation.
The local path caches successful applied values atomically for UI reads, invalidates
them on state load/rescan, and does not publish rejected pending batches. Ordered
mixed CLAP actions retain source order for source ticks sharing one sample; VST3
uses the native separate queue/sample precision rather than fabricating delay.

The ordered route also carries channel CC and pitch bend. CLAP queues MIDI events
only for a MIDI-capable port. VST3 uses a fixed, preparation-time table of existing
MIDI controller assignments and normalized values; it does not fall back to using
a CC number as a native parameter ID. Unsupported capabilities fail explicitly.
The fixtures expose CC7 and pitch bend through separate processor-observed values
and exercise a nonzero channel. This extension leaves the legacy MIDI path alone.
Controller classes remain distinct: an aftertouch assignment cannot substitute
for a missing pitch-bend assignment, even when an internal table uses a compact
slot for bend. A negative fixture exercises that boundary.

Suggested upstream direction: correct CLAP flags/global addressing, resolve
format-native IDs/cookies outside process, expose normalized VST3 scheduling
without controller/listener calls, and use bounded reusable parameter queues
with explicit error propagation. The attached local patch is generated outside
the checkout; its build composition is project-specific and is not proposed as
an upstream source layout.
