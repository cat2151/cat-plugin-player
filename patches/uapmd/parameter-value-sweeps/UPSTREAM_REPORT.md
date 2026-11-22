# Avoid unconditional CLAP VALUES rescan sweeps

Unsent report. Tested against UAPMD `4ec5c3b1c30130fdd3b7d4b375082273247e561f`.

`RemidyCLAPHost::paramsRescan(CLAP_PARAM_RESCAN_VALUES)` calls getParameter for
every parameter and notifies listeners. Six Sines (2554 parameters) requests this
inside state load and again via onMainThread. Each sweep takes about 0.7 seconds
in the tested installation, even when the host has no value-notification consumer.

Our local composition removes only the VALUES walk. Metadata rescans, main-thread
callbacks and parameter flushes remain. Explicit reads obtain the new values from
the plugin. A host with value-notification consumers needs a separate policy;
this is not a proposal to remove their updates unconditionally.

Consider making eager value notifications optional for hosts whose readers query
on demand. An isolated CLAP fixture counts get_value calls around state load and
the deferred callback and checks fresh explicit values and metadata invalidation.
