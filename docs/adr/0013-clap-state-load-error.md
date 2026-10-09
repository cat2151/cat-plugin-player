# ADR 0013: Propagate CLAP state load rejection through remidy completion

Status: accepted.

## Context

remidy logs CLAP state load=false but asynchronously completes with an empty
error. cat's shim already respects that callback, so UI/session/favorite
success can be false and an old instance can be discarded. Public owned patch
conversion alone cannot establish that the plugin applied the returned bytes.

## Decision

Add `patches/uapmd/clap-state-load-error/` as a fail-closed generated-source
repair after clap-note-dialect. Validate both the original state source and
the note-generated input; retain the generated class-header include; replace
exactly one remidy state source. Generate under default target/shim, without
editing the dependency checkout. Share a cpp-private main-thread loader,
return errors for missing owner/plugin/extension and false normal/context
loads, catch queued exceptions, complete once, and log for the legacy setter.
Invoke the normal extension directly after capability validation on the main
thread: clap-helpers' noexcept stateLoad cannot propagate exceptions to that catch.
Keep save, other formats, public interfaces and class layout unchanged.

## Consequences

Existing CLAP favorite/session paths now receive genuine load errors. Successful
paths remain unchanged; failure preservation is checked at native callback,
shim and Rust application boundaries with an isolated test CLAP. Existing note
behavior and real Surge normal state loading require regression verification.
Reclone and existing caches use the same configure order. Upstream source drift
stops generation with actionable diagnostics. Maintenance/removal conditions,
verified revision and unsent author report are recorded in the patch README.
No stderr parsing, audio-level success inference, checkout edits, other-format
repair or external report submission is included in this decision.
