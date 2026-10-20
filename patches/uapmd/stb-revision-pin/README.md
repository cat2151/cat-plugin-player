# stb revision pin

rtlog-cpp declares nothings/stb without GIT_TAG. On 2026-10-19 upstream moved
stb_sprintf.h into deprecated/, breaking rtlog's root include. Current upstream
main was inspected on 2026-10-20 and still has the unpinned declaration.
Verified local UAPMD: a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe;
release failure UAPMD: c4d5a590ddef42aa2695d29f02b7fa76b8cf7b1f;
rtlog-cpp: 36381f9b2d64f7852590dd0c2564dbfdee90e116;
stb: 2c980bb59875b0d32144a71867fbdebb2f77cd20.

shared/host.cmake registers the first FetchContent declaration before UAPMD.
rtlog retains population and target ownership; no dependency checkout is edited.
New clones fetch the pinned SHA. Existing caches are checked before population:
only a clean checkout at that SHA with the root header is accepted. Source
replacement and pre-existing targets are rejected. After UAPMD, population,
revision, header and exported include are checked again. An upstream structure
change fails with this repair's name, path, revisions and this document.

shared/build.rs watches this directory and uses configure stamp v4. The existing
CPM_SOURCE_CACHE is retained, avoiding repeat application of upstream patches.
Both app caches were inspected before enabling reconfigure. A mismatched cache
must be backed up (including modifications) before a separately authorized
recovery; do not delete or reset it to bypass this guard. UH_SKIP_CMAKE does not
validate or apply the repair. CRT policy is unchanged.

Run test_pin.py --work-dir <global-work-directory> --stb-source <clean-stb-checkout>.
The fixture uses real FetchContent and Git, mapping the public URL to the local
verified repository without downloading or editing it.

On upstream updates inspect rtlog's declaration, header use and target, verify
an upstream fix in the actual selected revision, then remove the pre/post include,
watch/stamp obligation, this directory and patch-list entries after both app
builds and regression checks pass. See ADR 0035.

Author report: **未報告**. UPSTREAM_REPORT.md is an unsent draft.
