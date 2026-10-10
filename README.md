# cat-plugin-player

A lightweight application for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- Audio plugin timbres are amazing!
- To play them, it often requires launching a DAW and going through setup procedures.
- Lightweight apps for easy playback sometimes require account registration or have a user experience that wasn't for me.
- Nowadays, if you have a desired UX, you can achieve it through vibecoding.
- So, I decided to do just that.

# Features
- Detection
  - Upon startup, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button loads the plugin and automatically starts playback.
- GUI
  - You can display the plugin's GUI by pressing the `Show UI` button.
- Timbre
  - To change the timbre, use the plugin's GUI.

※↑ This section is human-written. AI is prohibited from writing here.

# Features (Detailed additions by AI. Intended for later revision due to readability)

Oscilloscope display methods and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- GUI
  - The first time a GUI is opened, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG with its aspect ratio preserved, fitting the display frame of the list and the screen's zoom level. It saves to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers share the same image across VST3 / CLAP (ignoring leading/trailing spaces and case). If the name/manufacturer is missing, they are distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated and used as shared images based on the list/favorites identifiers. Timbre state and favorite load destinations are distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height, preserving the aspect ratio. Hovering over an image allows viewing it at its saved size. For plugins with no GUI displayed, a temporary icon of the same height is shown.
  - GUIs prioritize positioning to fit the screen's work area, aligned with the right and top edges of the main window. If it doesn't fit, it's placed in the bottom-right of the work area on the screen where the main window is located. It's positioned on display and when the GUI is resized, but does not constantly follow the main window's movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the work area; if its height is greater, its top edge aligns.
  - For blank images or failed captures, retries occur at 1-second intervals, up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured; opening the GUI will replace them with a valid image. After success, no further re-captures occur.
  - If capturing fails even after retries, an error is displayed. Opening the GUI again will retry. GUIs that do not support capture will remain with a temporary icon. If you want to retake an image or if a saved PNG is corrupted, delete the shared PNG for that plugin and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are kept during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next startup. Stop state, selected playback pattern while stopped, and confirmed MML/chord text are also restored. Unconfirmed edits are not saved.
  - CLAP Note On/Off is sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. The arrows cycle at the ends of the types, and "stop" is not included in the types. Switching during playback releases all notes in the current phrase and starts from the beginning of the new phrase. When stopped, selecting a type does not start playback; `Play` starts it and `Stop` stops it. The main window's `Space` key also toggles start/stop (except when typing text). The selection of MML/chord playback patterns and the stopped state are restored on next startup.
  - To audition MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). Examples are `t120 c8r8d4e4r4` for single notes and `C F G C` for chord progressions. Both are accepted in the same input field; chord notation conversion is attempted first. Capital `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter`/`Confirm` and canceled with `Esc`/`Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are used for text input, and `Space` does not play/stop.
  - Confirmed phrases loop with `Play`/`Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses normal overall playback. If playback was active just before opening, after confirmation, the new phrase plays from the start; after cancellation, the original playback pattern resumes from the start. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to normal playback patterns. The text, selection, and playback state are saved immediately after confirmation, and the restored playback state is also saved on cancellation. This is restored on next startup or from the sound source's Favorites/History. Unconfirmed text is not saved, and cancellation or conversion failure retains the last confirmed text.
  - Changing input or moving the cursor auditions the single note/chord at that position once. This also applies to mid-edit or pasting. Moving within the same pronunciation unit or merely redrawing does not re-trigger. It reflects the previous tempo, octave, and note length, and does not sound at rest or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the audition sound ends. The audition sound is released with the next pronunciation unit, cancellation, confirmation, or sound source exchange. Auditioning does not change the confirmed text or normal selection.
  - For confirmed MML loop playback, the `Velocity` field setting takes precedence over any velocity specified within the input. CC1 can also be controlled from the `CC1 modulation` field. If the event capacity is reached for very dense phrases, sounding notes are released, and playback stops. You can restart with `Stop` → `Play`.
  - When changing timbre via Favorites/History from the app, or via Random patch/Random effect, the stop/playback state is maintained. During playback, it plays from the beginning of the phrase and does not inherit any wait in the middle of the previous phrase. Leading rests specified in MML are preserved. Timbre changes made via the plugin's own GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is rebuilt due to loading, state restoration, or connection changes, it waits for the first phrase to start while continuing normal audio processing, to preserve initial sounds. At 48kHz, the wait is approximately 21ms for Six Sines and 85ms for TyrellN6. Leading rests in MML are added further. See [ADR 0019](docs/adr/0019-random-patch-all-catalog.md) for application conditions and validation.
  - Below the playback pattern, in the `Velocity` field, you can choose `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases values in Note On order within the phrase, then decreases in the next loop, repeating the action. Changes reflect from the next note without restarting the phrase, and are restored on next startup.
  - In the `CC1 modulation` field, you can choose `0`, `127`, `sweep` using the left/right arrows or dropdown. The initial value is 0. `sweep` repeats an action where it changes from 0 to 127 over 2 seconds, then from 127 to 0 over the next 2 seconds. The selection is also restored on next startup.
  - Guitar open strings send Note On for MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) sequentially at 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Off simultaneously, and it repeats after 0.5 seconds.
  - On successful `Load`, it's memorized when selecting sequence type, starting, or stopping. The memory remains after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - Upon exiting and `Remove`, binary states of timbre/parameters etc. exposed by the plugin via UAPMD's state API are saved per plugin to `%LOCALAPPDATA%\cat-plugin-player\states\`. On next startup or re-loading the same plugin, they are restored before playback starts.
  - `Load` for an instrument only replaces the sound source, maintaining connected effects and settings. It saves the state of the plugin being replaced, then switches after successfully generating, restoring, and connecting the new plugin. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. It does not include duplicating external samples or saving on forced termination.
  - On startup, it directly loads the previous instrument and effects, restores their states, and then starts playback. After that, it initializes the GUI and scans the list in the background.
  - If direct loading fails due to old history or file relocation, it restores from the scan results after GUI display. Old history is updated once played.
  - If the main sound source is CLAP Floe, when audio processing is rebuilt due to loading or timbre restoration, it delays the start of the first phrase by 100ms. Leading rests in MML are added further. This is a temporary measure for observing symptoms of disappearing initial sounds, and its resolution on actual hardware is unconfirmed. See [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - Serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects to the end. Connected effects can be removed one by one using `Remove`. Duplicate connections of effects with the same format + ID are not allowed.
  - `Random effect` next to `Sequence` draws a preset from installed CLAP effects from the same catalog and selection targets as cmrt's effect selector. If there are 0 effects, it adds the first one; if 1, it replaces that effect; if multiple, it replaces one of the connected effects equally. Repeated presses do not increase the number of effects, and maintain the order, settings, and Bypass state of other effects. To avoid duplicate connections, format + ID of already connected effects (other than the replacement target) are excluded from candidates. If no suitable candidates are available for the target slot, it displays a reason and does not change.
  - Random effect requires a sound source. Candidates are retrieved from the default effect/preset locations scanned by the play-server shared core, and no substitution to VST3 is made. During preparation, current playback continues, and loading, scanning, and timbre operations are disabled. If stopped, it remains stopped after application; if playing, it restarts the current phrase from the beginning, maintaining global MML, Velocity, and CC1 selections. In case of generation, application, or connection failure, it reverts to the original configuration, and no automatic re-draw of other candidates occurs. After success, the applied state and connection are saved for next startup, and no Favorites/History entries are added by Random effect itself. Saving failure is displayed as applied.
  - `Routing` displays the connection order. You can reorder effects by dragging the `↕` button up or down. During dragging, the moving effect name and insertion line are displayed. Instruments are not subject to reordering. You can edit timbre or effects using `Show UI` for each plugin. `Instrument`/`Effect` are displayed in the same type color as the list on the left.
  - Removing all effects reverts to a direct connection from the sound source to the output. `Bypass effect` targets the entire chain, allowing passthrough while retaining instance and settings. During Bypass, connected effects in Routing are displayed in gray, with `Bypassed` appended.
  - `Sequence` is for the instrument. Even if the Sequence is stopped, audio processing continues, allowing effect tails to decay.
  - On next startup, it restores the instrument, the order of all effects, their respective states, and Bypass status, then starts playback. History entries of old single effects can also be loaded. If effect restoration fails, an error is displayed, and it reverts to a direct connection from the sound source, but retains the saved configuration.
  - Add, remove, and reorder operations are saved only if the new connection is successfully prepared. If unsuccessful, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication; stereo → mono is an average of left and right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause brief audio glitches, and sounding notes and tails may be cut off. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Timbre
  - `Random patch` in `Sequence` equally draws from all CLAP patches of installed instruments that match format + ID (Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, Vaporizer2) found in the existing cmrt catalog. It's not an equal draw per sound source, so Dexed, with many candidates, will be selected more frequently. The tooltip displays the target sound source and total number of candidates. The button is not displayed if there are no compatible candidates.
  - The Random patch catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode. Random patch does not substitute VST3, draw effect timbres, or target unknown sound sources. Effect timbre drawing uses Random effect in the effect routing section. Dexed retains program specification within the cartridge. Floe/sforzando external samples are required at their original locations, and there is no function to duplicate/save samples.
  - During Random patch preparation, current playback continues, and conflicting load, scan, and timbre operations are disabled. It may wait for sample loading for Floe, etc. If stopped, it remains stopped after application; if playing, it restarts the current phrase from the beginning. Global MML, Velocity, CC1 selections, and effect order/state/Bypass are maintained. After success, the applied state is saved for next startup, and Random patch itself does not add to Favorites/History.
  - If preparation/application fails, it displays the reason and does not automatically draw another patch. It restores the original configuration, state, and playback state; instances where state restoration also failed will suppress saving. Saving failure after timbre application is displayed as applied. Application conditions and validation scope per sound source are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - You can save multiple timbres/settings as favorites using `★ Add favorite` for each plugin. For sound sources, playback patterns, stopped selections, and confirmed MML/chord text are also saved. For effects, playback settings are not saved. Names are automatically assigned and can be changed later.
  - Loading a different plugin automatically records the timbre, settings, and playback pattern of the plugin being replaced to History before switching. The name is `Plugin Name Sequence Number`. It is not added on reloading the same plugin or restoring on startup. Confirmed MML/chord text and stopped selection for sound sources are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is canceled.
  - During manual/automatic saving, it checks for duplicates (same plugin, role, playback pattern, confirmed MML text, state) and reuses existing entry names. In automatic History recording, only differences in stop state are treated as identical, but MML with different text is kept as a separate entry. Effects do not compare playback settings.
  - In [Plugin-specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md), you can check exceptions, reasons, scope, and limitations for each product. For confirmed state formats of TyrellN6 CLAP, the compressed part that changes only with playback is excluded from duplicate detection, and timbres with the same text timbre settings are grouped. Confirmed PCore `UI_op=9/10` and empty lines at the end of text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed Vaporizer2 CLAP format, tiny differences in MSEG time/coordinates that occur during loading/saving (absolute difference of specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) are treated as identical. Edits within this range cannot be distinguished. The saved data is kept as is.
  - Switch lists using the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order, with new additions placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON allows reordering using `↑`/`↓` at the end of the row and saving immediately. Loading by clicking a row and detaching effects work as before. `Sort by plugin name` sorts by plugin name in ascending order once and saves (case-insensitive, same-name items maintain relative order). Manual adjustments can still be made afterward. Reordering is disabled while filtering, and reorder mode is OFF on startup. See [Decision History and Specification (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is sorted by most recent registration. If determined to be the same timbre, the existing timbre is reused, its registration time is updated, and it is moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are maintained. All old format items are migrated to History, and all items not starting with `auto ` are also kept in Favorites (because old formats did not record the origin of automatic saves, to include all renamed automatic saves in History). See [Policy and Rationale (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus. Renaming/deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If there are CLAP plugins with the same name, manufacturer, and type, VST3s with identical names/manufacturers are hidden from the Plugins list (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are shown). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and maintained on next startup. It does not change Favorites or the format loaded/restored.
  - In both tabs, clicking the name/image loads; for connected effects, clicking again removes them. The currently loaded/connected row is highlighted, and the color is maintained even when effects are bypassed. `Load` / `Add` / `Remove` buttons in Plugins perform the same operations.
  - Favorites are displayed one entry per line. Long names/supplementary information are abbreviated; hover to see the full text. Waveform/spectrum are displayed below Routing on the right; the width of the right panel can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display between bottom-right/bottom of the screen (bottom-right on startup).
  - For sforzando CLAP's confirmed state format, it reuses existing favorites without creating new ones for only save counter differences and note/CC1 playback value differences. Differences in timbre settings are preserved. See [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - Each row shows a colored `Instrument` / `Effect` on the left. Clicking a favorite name restores its timbre/settings. For instrument favorites, it restores the saved confirmed text and playback pattern selection. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites without old format text retain current text, and old format stopped items retain current selection. If conversion or state loading/connection fails, the original text, selection, and playback state are restored, and the failed content is not saved to the session. Instrument favorites maintain connected effects, and effect favorites maintain the sound source and MML. A sound source is required to call an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed; clicking the name again removes that effect. This also applies if settings were edited after calling or during Bypass. The sound source, other effects, and saved favorites remain. Another timbre of the same format + ID switches settings at the current position, and unconnected effects are appended to the end. The selected favorite for each effect is also restored on next startup.
  - Rename/Delete can be accessed via `…` at the end of the row. Edits after calling or traditional automatic saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Saving/restoring may involve brief audio glitches, and the savable range is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` is unchecked for the oscilloscope, spectrum, oscilloscope, and Lissajous appear side-by-side at the bottom of the screen.
  - It draws the output of the past 50ms (up to 4096 samples), reflecting effects/Bypass, on a 45° rotated Lissajous. A common auto-gain preserves volume differences and aspect ratio; in-phase at the same level is a vertical line, out-of-phase is a horizontal line, and sine waves with phase differences are ellipses or circles. Left-only output is a diagonal from top-left to bottom-right, right-only output is a diagonal from top-right to bottom-left, mono is a vertical line, and silence is a central dot.
  - It updates regardless of notes, cycle counts, or trigger settings, and `Show analysis labels` can display headings. Axes and drawing methods can also be checked in the graph tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation over the past 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is out-of-phase. Negative values are shown in red, serving as a guideline for judging cancellation when mixed to mono. It shows "—" if silent or one side is silent.
- Waveform Display
  - `Oscilloscope` at the bottom of the screen displays the output's L (green) and R (blue) overlaid, reflecting effects/Bypass. Mono displays the same waveform on both.
  - The display width is determined from the MIDI note number of the last Note On (A4 = 440Hz), and you can choose `1`, `2`, `4`, `8 cycles`. After Note Off, the tail is displayed based on the last note. Output containing chords or detuning may not repeat perfectly at that width.
  - `Zero cross` aligns with negative → positive crossings of the left channel. If the left channel is silent, it uses the right channel; if there's no crossing, it displays from the beginning.
  - `Similarity` sweeps from the beginning of the buffer for the display width, one sample at a time, to find the position with the maximum correlation coefficient to the previous display. Ties prefer the earlier position, and the same starting position is used for both left and right. Zero cross is used for the first display or if the comparison target is silent.
  - The comparison history is reset when a note is sounded, playback pattern changes, display cycle count changes, trigger method changes, or the audio stream changes. It will show a waiting display until sufficient data is collected.
  - Exploration is performed in a separate thread from audio processing. Display updates may be slow for low notes or many cycles. If display data is interrupted, the history is reset.
  - Default display settings on startup are 4 cycles and zero cross. Display settings are not saved.

# Build & Operation Check Procedure
- ※Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its instructions. For Windows, it's easy if you have `Microsoft C++ Build Tools` installed! If not, just ask AI to install it – easy! The overall image is "easy by letting AI handle it."
- ※Next prerequisite: If Rust is not installed, it's easy to ask AI to install it!
- As mentioned above, ensure that `../uapmd` is built.
- Ensure audio plugins (instruments) like Surge XT are installed. For example, confirm Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm the screen opens.
- Confirm Surge XT, etc., are listed on the screen.
- Click the Load button.
- Confirm it loads and sound plays.

# Miscellaneous

The text generated by AI is hard to read. I'll fix it later.

## Window Position & Size Settings

In `%LOCALAPPDATA%\cat-plugin-player\config.toml`, you can specify the main window's startup position/size and the plugin GUI's placement method. Please edit it after closing the app. Session and favorite selection states are saved in `status.json` in the same directory, and normal saves do not overwrite the TOML. If the TOML is in a mixed state, it will be migrated on first launch: the original file is saved as `config.toml.before-status-migration.bak`, then only status items are removed. Configuration comments, formatting, and unknown items are retained, and independent comments attached to deleted items remain at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates; if both are specified, they are used as the startup position. If omitted, it uses the traditional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. Either can be specified alone; if omitted or set to 0 or less/non-finite values, it reverts to width 900, height 600. Changes will be reflected on next startup. The plugin placement method is currently `right_then_bottom_right` only; omitting it results in the same behavior. GUI size and placement are calculated based on actual window frame, screen work area, and DPI. Automatic saving of positions moved manually, or per-plugin position specification, is not performed.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this app's directory. TUI's `app` / `patches` / `patch-select` and play-server's `core-lib` / `server-config` are always locally referenced from Cargo.toml. Play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to the traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` requires their re-export. Random effect uses play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection criteria. Insufficient checkout or missing APIs will result in build errors, and there is no automatic switching to Git versions.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is located outside the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The UAPMD location for building is specified via an environment variable. `config.toml` is not read as a build setting.

Do not use `cargo install`; directly execute the `.exe` generated in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require re-configuration and re-building. Shim DLLs are bundled with the `.exe` and extracted to a cache directory for loading when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency structure and operational policy. Local switching of TUI is not required for building cat. If cross-building TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; 'off' is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. When verifying, record the HEAD and diffs of each repository.

Launches the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- ※Subject to change based on mood
- Extend strengths not present in cmrt, such as VST3 and GUI support
  - GUI automation?
    - Log GUI operations and reproduce quick TUI operations in cmrt? Provide hints for plugin-specific customization?
  - Experiment with egui
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates
  - ※This will also refine the crates, and we benefit from using them, so it's a win-win.
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can instantly play patches, effects, arpeggios, etc., specified from the CLI. Make the same possible (to the level where it plays just by changing cmrt to cat-plugin-player).
      - Frequent breaking changes are acceptable. Manual migration work is possible later. Better than being paralyzed by fear of change and hindering progress.

# Concepts, What We Aim For
- Instrument
  - For example, a synthesizer with a MIDI keyboard plays sound when powered on and a key is pressed.
    - We aim for that level of ease of use.
      - Minimize operations until sound plays.
- Playability
  - We aim to maintain "it plays in the author's environment" as much as possible.
- Immediate Playback
  - From app launch to sound playing, a perceived 0.2 seconds. Playback is prioritized for speed.
    - Processes like GUI display preparation are performed after the sound starts.
      - Sound playing before GUI display might be considered the audio version of a splash screen.
    - Requires direct execution of a release-built `.exe` (via `cargo` adds timestamp checks, and debug builds add debug code, both of which cause startup delays).
- Educational
  - It leans more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

# Random Thoughts Corner
- ※Related to concepts
- I love audio plugin preset timbres.
  - Because they are reproducible.
  - Because they are freely shared, benefiting everyone.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It is the synthesis of each layer from audio plugins to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and the performance UI.
        - This is a layer that can be vibecoded.
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - That is, it broadly targets beginners, including cats and babies (e.g., Android tablets).
  - ※This is purely a metaphor. Whether it actually runs on an Android tablet is unconfirmed. The likelihood of effort to make it run on Android if it doesn't is also low.

# Out of Scope, What We Don't Aim For
- Robustness
  - Absolutely no bugs, never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every possible feature. Usable for all purposes.
- Convenience
  - Extreme pursuit of convenience and ease of use.
- Performance
  - High-speed operation with no CPU load in any environment. Audio never drops out.
- Requests
  - Immediate response to all user requests.

# License
- MIT