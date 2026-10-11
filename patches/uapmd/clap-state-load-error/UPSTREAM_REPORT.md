# Draft: CLAP queued state load reports success after plugin rejection

Status: 未報告 / not submitted.

In UAPMD/remidy revision bc39b50faf2f2b02668de82d3d95f2bdbafd3c34,
`PluginInstanceCLAP::PluginStatesCLAP::loadState` queues `setState` and then
calls `finish("")`. The setter only prints an error when normal CLAP state.load
or draft state-context.load returns false. Missing state extension is also
reported as successful completion. The same pattern is present at
a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe.

Reproduction: instantiate a CLAP with a state loader that returns false, call
the asynchronous remidy state loader, and inspect the completion error. It is
empty although the plugin rejected the state. Hosts trusting that callback can
discard an old instance or persist a state that was not applied.

Proposed correction: share the main-thread load implementation in a cpp-private
helper returning a string error; call the helper from both the legacy void
setter (logging) and the queued loader (completion). Reject missing plugin or
state extension. Catch loader exceptions inside the queued main-thread task
and complete exactly once with the resulting nonempty error. Do not throw out
of EventLoop::runTaskOnMainThread, leaving a waiting caller incomplete. Preserve
save behavior, other formats and public class layout.
The normal extension call must also reach that catch; clap-helpers'
PluginProxy::stateLoad is noexcept, so our helper validates capability and
invokes the raw normal extension on the dispatched main thread, as for context.

The local patch has a separately built, unregistered test CLAP with normal,
context and missing-extension descriptors. The actual compiled backend tests
true/false and exception cases with callback counts; the shim and Rust native
tests exercise error propagation and favorite/session preservation. Generation
tests cover unchanged inputs and interaction with our existing note-dialect
generated class header. Patch sources are in
`patches/uapmd/clap-state-load-error/`; upstream source is never modified.
