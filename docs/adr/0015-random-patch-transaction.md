# 0015: Surge XT CLAP random patch transaction

Status: Accepted.

The first experiment chooses uniformly among eligible Surge XT CLAP patches.
The existing catalog reader owns plugin identity and display/path semantics;
`cmrt-patches::prepare_clap_patch_state` owns the Surge FXP to CLAP state conversion.
Other plugin formats and instrument-specific loaders remain outside this boundary.
Successful conversion does not establish native state-load or audible success.

An owned candidate crosses a detached preparation worker. The native host,
instances, state APIs and audio chain remain on the GUI thread. Preparation keeps
the current sound running and blocks competing action entrances without queuing
clicks. Catalog reading alone does not block those actions.

The pending load owns its purpose and state together (manual, restore, favorite,
or random). Random loading bypasses ordinary saved-state restoration and automatic
favorite capture. State backup occurs after preparation, immediately before native
mutation. A current-instrument failure restores state and transport; a replacement
retains the old instrument until the new state and chain succeed. Effects and their
order, bypass, velocity and modulation persist. Stopped playback starts the selected
sequence only on success. Existing Surge automatic note delay is reused.

Disk writes start after native success through existing state/session stores; no
new persisted schema or cross-file transaction is introduced. A saving failure
reports the sound as applied, with saving failed. Failed state rollback taints the
live instance and suppresses ordinary and Drop state writes, favorite capture and
session writes. Successful random recovery or explicit instrument reload clears
that memory-only taint. Failures do not select another patch automatically.

Acceptance uses pure candidate/worker tests and the compiled CLAP fixture with
limited transaction-operation failure injection. The fixture includes a separate
stereo effect identity so chain preservation and disk snapshots can be exercised.

実機SurgeはCLAP state.load成功後にaudio threadで音色を反映する。成功直後のnative saveは旧音色を返し得るため、random成功時は適用に使ったowned stateを既存state_storeへ保存する。追加の待機やprocessは行わず、通常の演奏・終了時保存は既存経路を使う。実機factory Bass 1→Bass 2のignored testで、直後の保存bytes、settled音色と音声差を確認する。
