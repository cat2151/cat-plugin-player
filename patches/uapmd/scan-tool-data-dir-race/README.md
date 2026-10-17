# PluginScanTool data-directory race

## Cause and bounded scope

Verified dependency revision: `a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe`.
`PluginScanToolImpl`'s constructor publishes its default directory into the
process-global `applicationDataDirectoryStorage()` path without a lock. Until
the path is first published, every constructor that runs concurrently takes the
writing branch, so hosts created on several threads at once write the same
`std::filesystem::path` together. The result is `STATUS_HEAP_CORRUPTION`
(0xc0000374) or an access violation in `uh_create`. Once published, later
constructors only read it.

The repair puts only that if/else under a function-local static mutex. The
public `applicationDataDirectory(path)` setter, the getter, the scanning code
and public API are unchanged. The application creates its first host on one
thread, so in ordinary use this occurs only in parallel tests.
See [ADR 0032](../../../docs/adr/0032-scan-tool-data-dir-race.md).

## Application and maintenance

The dependency checkout is never edited. CMake runs `apply.py` at configure
time; it validates the LF-normalized upstream SHA256, writes
`target/shim/generated/scan-tool-data-dir-race/PluginScanTool.cpp`, and makes
the copy's relative `../ipc/` includes absolute. CMake replaces exactly one
`src/scanner/PluginScanTool.cpp` in `uapmd-plugin-hosting`. Unknown upstream
changes fail closed with repair name, path, condition, revision and this README.
build.rs watches this directory and the upstream source.

If upstream has fixed this, verify the fix in the revision actually used, run
the parallel regression below without this repair, then remove this directory,
the include in `shared/host.cmake`, the build.rs monitors and the patch list
entries.

## Verification

```powershell
python patches/uapmd/scan-tool-data-dir-race/test_apply.py --source X:/uapmd/source/uapmd-plugin-hosting/src/scanner --work-dir X:/global-work/scan-apply
cargo build --release
cargo test --release --bin cat-plugin-player --no-run
# repeat the test executable 60+ times; every run must exit 0
target/release/deps/cat_plugin_player-<hash>.exe resident::tests::escape
```

Without the repair the parallel `resident::tests::escape` run fails in about
one run in ten; with it, 0 of 160.

## Author reporting

Status: **未報告**. [UPSTREAM_REPORT.md](UPSTREAM_REPORT.md) is an unsent local
draft. Creating it does not authorize external submission.
