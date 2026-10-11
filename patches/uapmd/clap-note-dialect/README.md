# CLAP input note dialect repair

## Cause and scope

Verified dependency revision: `a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe`
(CLAP sources and apply baselines are identical to the originally investigated
`bc39b50faf2f2b02668de82d3d95f2bdbafd3c34`). This revision converts every UMP
Note On/Off into native CLAP Note events and drops CC. Installed sforzando CLAP
2.1.2.4 reports input port 0 supported/preferred dialect `2` (MIDI only).
The repair selects the input port's supported dialect: native CLAP when supported,
otherwise MIDI when supported, otherwise no unsupported note event. Both-capable
ports receive one native note, never duplicate native/MIDI notes. Missing note-port
extension retains the previous native-note behavior. Invalid ports and failed port
queries do not receive guessed events.

Capabilities are collected on the main thread before activation in configure;
the process callback reads the cached vector without querying plugin extensions.
UMP group maps to note-port index, as before. Channel, key, port and timestamp match
between On/Off. MIDI1 velocities round-trip via UAPMD's `velocity << 9`; positive
MIDI2 velocities below one MIDI1 unit map to 1 instead of accidental Note Off.
CC1 and CC120 use MIDI only on MIDI-capable ports, including both-capable ports.
Native-only ports retain explicit Note Off; this patch does not invent native
modulation or broaden to other CC, MIDI2 or MPE. See [ADR](../../../docs/adr/0005-clap-input-note-dialect.md).

## Application and lifecycle

Configure requires a Python 3 interpreter, discovered by CMake.
`apply.py` validates normalized LF SHA256 baselines of the header, instance and
events implementations before writing any outputs. Equivalent CRLF checkout
inputs are accepted. It generates copies under the default
`target/shim/generated/clap-note-dialect/`; the dependency checkout is never edited.
All nine translation units using the modified class header are copied with explicit
generated-header includes to avoid differing class layouts across translation units.
Other local includes are redirected to original dependency headers.

The shim CMake include replaces exactly one remidy source per generated translation
unit. `build.rs` watches patch files and upstream CLAP input sources. CMake watches
the same inputs and reapplies on regeneration. Identical generated contents retain
their timestamps. A new dependency clone matching these baselines can generate the
same repair; rebuilding after cache loss uses the regular upstream dependency
procedure. This is source-generation reproducibility, not a claim that the whole
dependency tree has been successfully rebuilt in a new environment.

If application fails, stop and read the reported repair name, source, condition,
UAPMD revision and this README path. Compare the current input with the documented
revision and the hashes in `apply.py`. Do not bypass checks or change the pinned
dependency to make the repair apply. Confirm an upstream correction before updating
baselines or removing the CMake include, build monitoring and repair directory.
Removal requires reproducing native/MIDI selection, single emission and the native
audio cases below without this local repair.

## Validation

From the project root, use the default target and run serially:

```powershell
python patches/uapmd/clap-note-dialect/test_apply.py --source X:/uapmd/source/remidy/src/clap --work-dir X:/global-work/clap-apply-test
cargo build --release
cmake --build target/shim --config Release --target clap_note_dialect_test clap_note_native_ab
target/shim/Release/clap_note_dialect_test.exe
```

The application test's work directory must be new; artifacts are retained as
evidence. The policy test checks MIDI-only/native/both/unsupported dialects,
single emission, On/Off fields, timestamps, positive velocity and CC1/120.
The application test checks fresh input generation, deterministic reapplication,
unchanged input, no partial writes on upstream drift and stop diagnostics.

`clap_note_native_ab.exe <plugin> <state.bin> native|midi off|held` is an independent
direct CLAP host. Each invocation uses a fresh instance and restores the state
bytes through an in-memory stream. It records port capability, plugin version,
state size, equal 48 kHz/256-frame processing, CC1=0, note 60/velocity 100/channel 0,
Note On at sample 256 and Note Off at sample 48128. RMS/peak cover the sounding
period, early release and late release; process-status counts are recorded.
The held arm distinguishes release caused by Note Off from natural sample decay.
Output is measured in memory; no audio device, product config, state or favorites
are written. Direct native/MIDI comparison is separate from the repaired app path.

`native_app.py --root <project> --plugin <plugin> --id <CLAP-id> --name <name>
--state <state.bin> [--held | --shutdown]` runs the same timings through the actual shim's UMP
processor. Use separate invocations for off and held controls. A native-capable
instrument regression must also be measured. Human GUI-keyboard versus host
playback acceptance remains separate; VST3 results do not verify CLAP.
`--shutdown` uses the application's 128 channel-0 Note Off messages followed by
CC120 on all 16 channels, rather than a single musical Note Off.

Verified on 2026-10-07 with the revision above and saved TableWarp2 state (421 bytes,
SHA256 `4931169322f524938bb7fdd6352b3835eb7c284e16517078c61514076d3938c2`):

| Path / event | Sounding RMS / peak | Early tail RMS | Late tail RMS |
|---|---|---|---|
| Direct native Note | 1e-20 / 1e-20 (plugin silence floor) | 1e-20 | 1e-20 |
| Direct MIDI, Note Off | 0.0758554 / 0.272482 | 0.00666599 | 1e-20 |
| Direct MIDI, held | 0.0758554 / 0.272482 | 0.0180376 | 4.84215e-6 |
| Repaired app UMP, Note Off | 0.0758554 / 0.272482 | 0.00666599 | 1e-20 |
| Repaired app UMP, held | 0.0758554 / 0.272482 | 0.0180376 | 4.84215e-6 |
| Repaired app, Surge XT native-capable default | 0.0800660 / 0.212675 | 0.00514413 | 0 |

Each direct arm returned `CLAP_PROCESS_CONTINUE` for all 1500 blocks; shim arms
returned success for all 1500 blocks. Equal off/held sounding intervals and divergent
tails demonstrate that Note Off is delivered while allowing the tone's release.
Surge XT 1.3.4 reports supported dialects 7/preferred 1, so it exercises the native
branch. Product config, favorites index and saved tone bytes remained unchanged.
Policy/application tests and the ordinary release build succeeded using existing
caches. CMake's pre-existing repeated ImTimeline patch failure required restoring
only generated cache files whose Git blobs exactly matched the upstream patch's
postimages; no build/cache setting or upstream checkout changed. This is not a
fresh-environment dependency-build result. Human GUI-keyboard/host listening (H04)
and VST3 observations remain unconfirmed.
The shutdown-batch arm also passed all 1500 calls and reached the same late
silence floor; no immediate-release assumption was imposed. `cargo fmt --check`
and `cargo clippy --all-targets -- -D warnings` passed. Whole-project unit and
other stage acceptance checks belong to the separate final handoff verification.

## Author reporting

Status: **未報告**. [UPSTREAM_REPORT.md](UPSTREAM_REPORT.md) is a local draft;
preparing it does not authorize sending it upstream.
