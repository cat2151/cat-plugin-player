# ADR 0005: Select CLAP note input dialect by capability

Status: accepted

## Context

sforzando CLAP declares MIDI-only note input while the current UAPMD dispatcher
emits native CLAP Note On/Off unconditionally. Choosing MIDI1 UMP in Rust cannot
fix this conversion. Naming one plugin in the host would leave the same mismatch
for other MIDI-only instruments. Changes must preserve the dependency checkout,
fail visibly on changed upstream assumptions and include regression evidence.

## Decision

Cache per-input-port supported dialects on the main thread before activation.
Prefer native CLAP on native-capable ports and MIDI on MIDI-only ports. Send exactly
one note event, preserving port/channel/key/timestamp and positive velocity. Keep
the previous native fallback when no note-port extension exists; do not send to
explicitly unsupported or invalid ports. Send only currently used CC1/CC120 through
MIDI-capable ports; retain explicit native Note Off on native-only ports.

Generate a validated modified header and consistent copies of the nine CLAP
translation units which include it under the default `target/shim`. Rewrite local
includes to avoid silently picking the original header, then replace remidy's
compile inputs in CMake. Validate the three modified inputs before any output is
written; monitor inputs in cargo/CMake for regeneration. Do not patch the checkout.

## Consequences and acceptance

No plugin-name special case, new MIDI2/MPE feature or general CC implementation is
introduced. Capabilities remain stable for the configured activation; queries are
absent from the audio callback. Generated class layouts agree across translation
units. Upstream changes require explicit investigation instead of silently skipping
the repair. Port remapping or broader dynamic dialect handling is outside this fix.

Controlled fresh-instance same-state native/MIDI audio comparison must establish
the observed mismatch's effect. The repaired app path must produce nonzero audio,
with Note Off compared against a held-note control to respect release tails, and a
native-capable instrument must retain sound. Policy/application regressions and
normal project build checks are required. Human GUI and playback acceptance is
not inferred from numeric measurements. See the repair README for lifecycle,
failure investigation, removal conditions and author report status.
