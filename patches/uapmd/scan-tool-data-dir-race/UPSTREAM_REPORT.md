# Draft: concurrent PluginScanTool construction corrupts the heap

Status: 未報告 / not submitted.

In UAPMD revision a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe,
`PluginScanToolImpl::PluginScanToolImpl()` (`src/scanner/PluginScanTool.cpp`)
does:

```cpp
if (auto& supplied = applicationDataDirectory(); !supplied.empty())
    dir = supplied;
else if (!dir.empty())
    applicationDataDirectory(dir);
```

`applicationDataDirectory(path)` assigns a function-local static
`std::filesystem::path` without synchronization. While it is still empty,
constructors running on different threads all take the `else` branch and
assign the same path concurrently. On Windows this shows up as
`STATUS_HEAP_CORRUPTION` or an access violation inside
`AudioPluginHostingAPI::create()`.

Reproduction: start three threads behind a barrier and have each call
`AudioPluginHostingAPI::create()` (through the remidy host) at the same time in
a fresh process. In our runs about 2% of processes crash; a test suite that
creates a host per test in parallel crashed in about 10% of runs. Creating one
host before going parallel makes it disappear, since afterwards the branch only
reads.

Proposed correction: perform that read-or-publish under a static mutex (or
`std::call_once`). Separate host objects on separate threads then stay
independent, as callers would expect from a library without a documented
single-thread initialization requirement.
