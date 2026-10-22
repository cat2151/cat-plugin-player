# Direct parameter automation

## Cause and scope

Verified UAPMD revision: `a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe`.
The CLAP metadata constructor swaps automatable/readable and ignores read-only;
its global RT event addresses note/key/port/channel zero and sets LIVE. VST3's
plain RT setter calls the controller and a mutex-backed listener, retains a
nonmatching queue when searching for another ID, and allocates/destroys queues.
These paths cannot safely implement direct curve automation.

The repair corrects CLAP metadata/global addressing and provides a separate
resolved automation path. CLAP accepts plain values. VST3 accepts normalized
values directly into fixed storage (256 IDs, 64 points per ID). It does not
call the controller or parameter listener to apply those values. Metadata,
current values and plugin formatting remain UI operations. Stepped/list,
hidden, read-only and nonautomatable parameters are not eligible. Text-to-value
is not exposed; clients must distinguish native numeric input from formatted text.

The existing VST3 plain/MIDI API is retained; this patch does not claim to repair
all legacy parameter or MIDI paths. Automation mode suppresses the existing
output-parameter listener/controller conversion path. Output UMP still uses the
plugin's normalized output. New automation input is not serialized as patch state.
See [ADR 0037](../../../docs/adr/0037-direct-parameter-automation.md).

## Application, failure and removal

The dependency checkout is never modified. Configure first generates the existing
CLAP note-dialect and state-load repairs, then validates their complete output
against freshly generated expectations. Parameter generation uses one coherent
class header per backend for every related translation unit. CMake replaces
exactly one source per backend/file and rejects unexpected or duplicate inputs.
Output is under each application's default `target/shim/generated/parameter-automation/`.

Original modified inputs use LF-normalized SHA256 checks, accepting CRLF. Changed
upstream or composed inputs stop before writing outputs. Diagnostics identify
the patch, target, condition, revision and this README. Identical regeneration
keeps output content/timestamps. New clones and existing caches use the same
configure order; the cargo configure stamp and source watches trigger the path.
No replacement target directory or dependency cache modification is required.

On failure, inspect the named upstream/composed input and its diff against the
verified revision. Do not relax hashes, skip the patch or use an old DLL. If
upstream fixes the underlying behavior, verify metadata, two-ID delivery,
normalized values, RT allocation bounds and note/state regressions on that
revision before removing this patch, CMake include, build watches, configure
stamp requirement and AGENTS patch-list entry. Class-header composition must
remain consistent if any part is retained.

The shim rejects stale instance generations and metadata revisions, duplicate
indices, invalid values and capacity excess. Preparation requires stopped audio
renderers and resolves ID/cookie/range information before rendering. One accepted
batch must immediately precede process; failure must stop playback, not be ignored.
Processors must be destroyed before their instance/host. Metadata rescans require
re-preparation and fresh bindings; persisted stable IDs must be resolved again.

The ordered C ABI accepts at most 512 mixed note/parameter actions. CLAP keeps
source order even when several source ticks map to one sample; VST3 keeps native
sample precision in its separate queues. This does not emulate sub-sample timing.
VST3 UI current values read an atomic applied-value cache committed after successful
processing. State load and metadata changes invalidate it. Overflow/rejected batches
cancel pending values and must not change the reported current value.

## Verification

Run serially from the application root. Tests use only explicit isolated bundles
under `target/shim`; do not register/install them or scan the user's catalog.
For the editor, prefix patch paths with `../cat-plugin-player/`.

```powershell
python patches/uapmd/parameter-automation/test_apply.py ../uapmd/source/remidy/src target/shim/generated/clap-note-dialect target/shim/generated/clap-state-load-error X:/global-work/automation
cmake --build target/shim --config Release --target parameter_clap_fixture parameter_vst3_fixture parameter_backend_test uapmd_shim clap_state_test_plugin clap_state_backend_test clap_note_dialect_test
target/shim/Release/parameter_backend_test.exe target/shim/Release/parameter_clap_fixture.clap target/shim/Release/parameter_vst3_fixture.vst3 fixed
python patches/uapmd/parameter-automation/test_abi.py target/shim/out/uapmd_shim.dll target/shim/Release/parameter_clap_fixture.clap target/shim/Release/parameter_vst3_fixture.vst3
target/shim/Release/clap_state_backend_test.exe target/shim/Release/clap_state_test_plugin.clap
target/shim/Release/clap_note_dialect_test.exe
```

The baseline fixture recorded both backends failing processor-observed values/order;
CLAP also exposed wrong flags. The fixed test checks two independent IDs before
Note On, a separate instance, reconstruction, plugin formatting and normalized
VST3 values with a distinct nonlinear plain range. The ABI test checks eligibility,
generation/revision invalidation (including a real CLAP rescan), all-before-enqueue
validation, duplicates, ordered same-sample actions and capacity errors. The VST3
current-value check also rejects publishing a cancelled overflow batch. Backend storage bounds and host
allocation checks are separate from audio-device/subjective listening checks.
Source regeneration against unchanged inputs is not a clean-environment build.

## Author reporting

Status: **未報告**. [UPSTREAM_REPORT.md](UPSTREAM_REPORT.md) is an unsent local draft.
