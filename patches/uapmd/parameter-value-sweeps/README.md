# Parameter value sweeps

## Cause and scope

Verified UAPMD revision: `277aa85468795267d6cf64953af587ea18c7f653`.
Six Sines requests VALUES rescans both while loading state and from its queued
main-thread callback. Each rescan walks every current value just to notify listeners.
This shim observes metadata changes, but has no consumer of those value notifications.
Current-value readers query the plugin on demand. Remove only that walk, keeping
ALL, INFO and TEXT metadata rebuilding, request_callback and request_flush.
The player's macro discovery separately uses metadata without querying current values;
the existing `uh_parameter_info` contract still reads a current value.

## Application and failure

CMake composes this repair after parameter-automation. It regenerates that repair's
expected input and rejects any changed upstream or composed input, then replaces
exactly one remidy source with `target/shim/generated/parameter-value-sweeps/HostClasses.cpp`.
Absolute includes retain parameter-automation's coherent class definition.
Neither the dependency checkout nor existing repairs are rewritten. Reclone and
existing caches use the same CMake include and cargo source watch; unchanged generation
preserves output timestamps. No alternate target directory is needed.

On failure, diagnostics identify the repair, target, condition, UAPMD revision and
this document. Inspect the named source and upstream/composition differences; do not
relax validation, skip the repair or reuse an old DLL. If upstream removes the sweep,
verify current reads, metadata invalidation, callbacks and flushes on that revision,
then remove this directory, its host.cmake include and build.rs watch together.
Value-notification consumers added later must explicitly refresh their values after
state changes; this repair does not maintain such a cache.

## Verification

From the project root, serially:

```text
python -B patches/uapmd/parameter-value-sweeps/test_apply.py
cmake --build target/shim --config Release --target uapmd_shim parameter_clap_fixture parameter_vst3_fixture
python -B patches/uapmd/parameter-automation/test_abi.py target/shim/out/uapmd_shim.dll target/shim/Release/parameter_clap_fixture.clap target/shim/Release/parameter_vst3_fixture.vst3
```

The ABI test uses the existing isolated parameter fixture to count get_value,
metadata, callback and flush calls; tests VALUES loads and deferred callbacks,
fresh explicit current reads, INFO/ALL/TEXT invalidation, and the unchanged public API.
Composition tests cover drift, missing/duplicate anchors, CRLF, unchanged inputs,
and idempotence. This is existing-cache verification, not a fresh-environment build.

For the application path, run `python scripts/measure_patch_browser.py --help`.
Supply the installed Six Sines CLAP file with `--bundle`, its factory directory
with `--factory`, and a directory outside the repository with `--output`.
The opt-in native test dispatches A-B-A browser requests through the real worker,
transaction and persistence code. A test-only sink retains that transaction's
processor and uses the common sequencer, macro sender and native processing API.
Its JSON records preparation, backup/load/route/persistence, longest headless egui
dispatch, first nonzero memory audio, first new-patch audio, distinct/returning
sound and restart checks. New-patch onset uses the first block matching the
stable first-note brightness, checked against distinct B and returning A.
It has 1024-frame resolution and excludes a previous release/queued buffer;
raw first signal is reported separately.
It uses the default debug Rust test build and Release shim, 48kHz/1024 frames,
stereo Steps at velocity 100 without effects. It opens neither a window nor an
audio device; cpal callback delivery, OS event delivery and listening remain
separate human checks. Instrumentation is compiled only for tests.

## Author reporting

Status: **未報告**. [UPSTREAM_REPORT.md](UPSTREAM_REPORT.md) is an unsent draft.
