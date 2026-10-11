# CLAP state load error propagation

## Cause and bounded scope

Verified dependency revision: `a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe`
(CLAP sources and apply baseline are identical to the originally investigated
`bc39b50faf2f2b02668de82d3d95f2bdbafd3c34`). It still calls `setState(...)`
followed by `finish("")` even when CLAP state.load returns false. The shim already propagates callback errors;
the missing link is remidy's CLAP backend. This can incorrectly commit a
replacement instance or save a patch that the plugin rejected.

The repair changes only state loading in `PluginInstanceCLAP.States.cpp`.
A translation-unit private helper performs main-thread normal/context loads.
Missing owner/plugin or state extension, missing context load, and false return
produce a nonempty error. Only true is success. The queued loader catches
standard and unknown exceptions on the main thread and completes once, outside
the exception-catching block. The legacy void setter logs errors. State save,
other formats, public API and class layout are unchanged. The constructor
initializes its extension pointer before the missing-owner/plugin guard.
Existing CLAP favorite/session restoration now sees genuine load failures.
The normal extension is invoked directly after the proxy's capability check:
the proxy's `stateLoad` is `noexcept` and would terminate on a thrown plugin
exception before the queued catch could translate it. Main-thread dispatch
is retained for both extension paths.
See [ADR 0013](../../../docs/adr/0013-clap-state-load-error.md).

## Application and maintenance

The dependency checkout is never edited. CMake first runs clap-note-dialect,
then this repair consumes its generated States.cpp and preserves the absolute
include of the note-generated class header. Output is only the default
`target/shim/generated/clap-state-load-error/PluginInstanceCLAP.States.cpp`.
CMake replaces exactly one note-generated source and rejects another state
translation unit; the original and both copies are never compiled together.

`apply.py` validates the LF-normalized upstream SHA256 and the complete expected
note-generated input before writing. CRLF is accepted; unknown upstream/input
changes fail closed with repair name, offending path, condition, dependency
revision when available and this README. Identical reapplication preserves the
output timestamp. build.rs watches this repair directory and upstream CLAP
sources; CMake watches the upstream/generated input and apply.py. A new clone
matching the baseline follows ordinary configure, note repair, state repair;
existing caches reconfigure through the same path. No checkout rewrite or
special cache edit is part of this repair.

On failure, read the full diagnostic, compare original and note-generated
inputs against the verified revision and inspect the upstream change/fix.
Do not bypass the baseline, silently skip the repair, or substitute an old DLL.
If upstream has fixed this, verify the fix in the revision actually used,
then repeat backend callback, shim/Rust, note and favorite/session regressions
before removing this directory, CMake include, build monitor and patch list
entry. Updating an uncorrected baseline requires a source review first.

## Verification

Run serially from the project root, using a new global evidence directory for
temporary fixtures (examples use fictional X-drive paths):

```powershell
python patches/uapmd/clap-state-load-error/test_apply.py --source X:/uapmd/source/remidy/src/clap --work-dir X:/global-work/state-apply
python patches/uapmd/clap-note-dialect/test_apply.py --source X:/uapmd/source/remidy/src/clap --work-dir X:/global-work/note-apply
cargo build --release
cmake --build target/shim --config Release --target clap_state_test_plugin clap_state_backend_test clap_note_dialect_test
target/shim/Release/clap_state_backend_test.exe target/shim/Release/clap_state_test_plugin.clap
target/shim/Release/clap_note_dialect_test.exe
$env:S03_NATIVE_WORK_DIR = 'X:/global-work/native-state'
cargo test native_clap_state_error_and_favorite_session_preservation -- --ignored --test-threads=1 --nocapture
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Application regression checks fresh input, LF/CRLF, nonmutating reapplication,
unchanged timestamps, upstream/input drift diagnostics, preserved note header
and the actual ordered CMake single-source replacement. The isolated test CLAP
bundle is built only under target/shim and loaded by explicit bundle path. It
must never be installed or registered in the user's plugin catalog. Native
backend regression counts queued callbacks and checks normal/context true,
false, standard/unknown exceptions and missing extension. Rust regression uses
the shim and checks Err, successful favorite/session restoration, failed
favorite rollback with immutable disk data, and rejected new-slot session
restoration keeping the old instance and pending cleanup.

Source-generation reproducibility with a simulated fresh input is separate
from a clean dependency build. Actual build/native outcomes are recorded in
the handoff execution evidence; an existing-cache build does not establish
clean-environment dependency reproducibility. Real Surge XT normal state
loading is a separate native acceptance check.

## Author reporting

Status: **未報告**. [UPSTREAM_REPORT.md](UPSTREAM_REPORT.md) is an unsent local
draft. Creating it does not authorize external submission.
