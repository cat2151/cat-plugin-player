# ADR 0037: Separate resolved automation from live parameter editing

Status: accepted

## Context

The editor needs two independent curves to control parameters of one existing
instrument instance. MIDI CC mapping would make parameter selection depend on
plugin configuration; enumerated stable IDs identify the actual exposed targets.
The available plain-value RT API initially looked suitable for scheduling.

Processor-level fixtures showed that this assumption fails: CLAP swaps metadata
flags and addresses global values to note zero; VST3 can put the second ID's
value into the first queue. The VST3 path also converts through a controller,
notifies mutex-backed listeners and allocates queue storage. Merely wrapping that
API would preserve both wrong targets and audio-thread work.

## Decision

Keep UI discovery, native value formatting and stable-ID resolution outside the
audio callback. Apply prepared global targets through a separate bounded route:
CLAP plain values with global addressing, VST3 normalized values without controller
conversion or listener notification. Preserve the legacy plain/MIDI API rather
than silently redefine its units. Reject stale generations/revisions, invalid
values, conflicting indices and capacity excess instead of dropping updates.

Compose a validated generated-source repair after the existing CLAP note/state
repairs, keeping one class definition across all backend translation units. The
dependency checkout remains unchanged, and upstream/composition drift stops
generation. A direct edit of downloaded sources would be lost on reclone and
would make existing-cache success conceal missing build inputs.

## Runtime ordering and current values

Different source ticks can round to one audio sample. Sorting every parameter
before every note would change the musical order, so the bounded route also
accepts already ordered mixed actions. CLAP preserves that order within its event
list; VST3 retains the format's separate event/parameter queues and sample precision.
No extra sample delay is invented to separate them.

Pattern expression also needs channel CC and pitch bend beside those prepared
parameter values. The original ordered route accepted notes only; sending controls
through the separate legacy path would lose their position relative to expression
events and would retain VST3's controller-based scheduling. Extend the bounded
route with MIDI controls. CLAP uses the input port's cached MIDI capability;
VST3 resolves the existing channel/controller assignments during preparation and
uses a fixed table while processing. An absent mapping rejects delivery instead
of guessing that a CC number is a parameter ID. This preserves the format's native
queue precision and leaves the legacy API's behavior unchanged.

VST3 controller values do not track processor-applied automation. Reading only the
controller after playback returned the old value, while retaining an applied cache
after loading baseline state returned the last automated value. Therefore UI current
values use a lock-free applied-value cache, invalidated by state load and metadata
changes. Values become visible only after a successful process call; rejecting an
oversized batch must not publish values the processor never received.

## Consequences

The stereo fixture receives 0.7/0.8 before Note On on both backends while another
instance retains 0.2/0.3. A nonlinear VST3 plain range guards the chosen normalized
contract. These observations establish delivery rather than perceived smoothness;
the editor still has to schedule from audio time and verify listening separately.
Automation is opt-in so ordinary startup does not enumerate all parameters.
Metadata rescans invalidate bindings and require preparation with stopped renderers.
Storage limits are explicit failures, not permission to omit curves. Maintenance,
test procedures and the unsent author report belong to the
[patch documentation](../../patches/uapmd/parameter-automation/README.md).
