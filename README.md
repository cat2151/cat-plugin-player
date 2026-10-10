# cat-plugin-player

A lightweight app that makes it easy to play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Challenges Addressed by This App
- Audio plugin timbres are fantastic!
- To play them, it often requires launching a DAW and going through setup procedures.
- Lightweight apps for easy playback sometimes require account registration to obtain, or their operation feel was "not for me."
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So, that's what we decided to do.

# Features
- **Detection**
  - Upon launch, it automatically detects installed plugins.
- **Playback**
  - Pressing the `Load` button automatically starts playback after loading the plugin.
- **GUI**
  - Pressing the `Show UI` button displays the plugin's graphical user interface.
- **Sound Tones**
  - To change sound tones, use the plugin's GUI.

*↑This section is written by a human. AI is prohibited from writing here.*

# Features (Detailed additions by AI. Will be refined later due to poor readability)

Oscilloscope display methods and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- **GUI**
  - The first time the GUI is opened, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG image that fits the list display frame and screen zoom level while preserving its aspect ratio. It is saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`. Plugins with matching names + manufacturers (ignoring leading/trailing spaces and case) share the same image across VST3 / CLAP. If name/manufacturer is missing, it distinguishes them by format + ID.
  - Previously saved format-specific images are also automatically migrated and used as shared images based on list and favorite identification information. Tone states and favorite load destinations are still distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while preserving the aspect ratio. Hovering over an image displays it at its saved size. Plugins with no GUI displayed show a temporary icon of the same height.
  - The GUI prioritizes placement to the right of the main window, aligned with its top edge, and within the screen's working area. If it doesn't fit, it's placed in the bottom right of the working area of the screen where the main window is located. It's positioned upon display and when the GUI size changes, but it doesn't constantly track the main window's movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the working area; if its height is greater, its top edge aligns.
  - For blank images or failed captures, it retries at 1-second intervals for up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured, and opening the GUI replaces them with valid images. After a successful capture, it won't re-capture.
  - If capture fails after retries, an error is displayed. Opening the GUI again retries the capture. GUIs that don't support capture remain with a temporary icon. If you want to retake a screenshot or if the saved PNG is corrupted, delete the shared PNG and any remaining old format-specific PNGs for that plugin, restart the app, and open the GUI (old PNGs are kept during migration).
- **Playback**
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on the next launch. Stopped state, selection of stopped playback patterns, and confirmed MML/chord content are also restored. Uncommitted edits are not saved.
  - CLAP Note On/Off messages are sent according to the input format supported by the plugin. This also applies to sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using the left/right arrows or the dropdown. The arrows cycle at the ends of the types, and "Stop" is not included in the types. Switching during playback releases all notes in the current phrase and starts from the beginning of the new phrase. When stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except when typing). Selection of playback patterns including MML/chords and stopped state are restored on next launch.
  - To try out MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). Examples: `t120 c8r8d4e4r4` for single notes, `C F G C` for chord progressions. Both are accepted in the same input field, with chord notation conversion attempted first. Uppercase `C` indicates a chord, lowercase `ceg` indicates MML.
  - Input is confirmed with `Enter` / `Confirm` and canceled with `Esc` / `Cancel`. Cancellation or conversion errors revert to the previous valid phrase. `Space` and `i` within the input screen are used for typing, and `Space` for play/stop does not work.
  - Confirmed phrases loop with `Play` / `Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses normal overall playback. If playback was active just before opening, after confirmation, the new phrase plays from the beginning; after cancellation, the original playback pattern resumes from the beginning. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to normal playback patterns. The content and selection/playback state are saved immediately after confirmation, and the restored playback state is also saved upon cancellation. This is restored on next launch or from sound source favorites/History. Uncommitted content is not saved, and cancellation or conversion failure retains the previous confirmed content.
  - Changing input or moving the cursor triggers a one-time audition of the note/chord at that position. This includes mid-edit and paste operations. Moving within the same phonetic unit or redrawing alone does not retrigger the sound. It reflects previous tempo, octave, and note length, and does not sound at rest or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the audition sound ends. The audition sound is also released on the next phonetic unit, cancellation, confirmation, or sound source change. Auditioning does not alter the confirmed content or normal selection.
  - For loop playback of confirmed MML, the `Velocity` field setting takes precedence over any velocity specified within the input. CC1 can also be controlled from the `CC1 modulation` field. If a very dense phrase reaches the capacity of playback events, sounding notes are released, and playback stops. It can be restarted with `Stop` -> `Play`.
  - When changing tones via app Favorites/History or Random patch, the stop/play state is maintained. During playback, the phrase starts from the beginning and does not carry over any wait periods from the middle of the previous phrase. Leading rests specified in MML are preserved. Tone changes made through the plugin's own GUI are not detected.
  - In the `Velocity` field below the playback pattern, you can choose `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases values in Note On order within the phrase, then decreases them in the next loop, repeating this behavior. Changes are reflected from the next note without restarting the phrase and are restored on next launch.
  - In the `CC1 modulation` field, you can choose `0`, `127`, `sweep` using the left/right arrows or dropdown. The initial value is 0. `sweep` repeatedly changes from 0 to 127 over 2 seconds, then from 127 to 0 over the next 2 seconds. The selection is restored on next launch.
  - Guitar open strings send MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) sequentially at 125ms intervals. 2 seconds after the last Note On, all 6 notes are simultaneously Note Off, then repeated after 0.5 seconds.
  - Saved on `Load` success, and when selecting/starting/stopping a `Sequence` type. The memory remains after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, the plugin's binary state (tone, parameters, etc.) exposed by UAPMD's state API is saved to `%LOCALAPPDATA%\cat-plugin-player\states\` per plugin. On next launch or re-load of the same plugin, it's restored before playback starts.
  - `Load` for an instrument replaces only the sound source, maintaining connected effects and settings. It saves the state of the plugin being replaced, then switches after successfully creating, restoring, and connecting the new plugin. If it fails, it retains the original configuration and displays an error.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. It also requests UI state, but the current UAPMD VST3 implementation only retrieves component state. It does not include external sample replication or saving on forced termination.
  - On launch, it directly loads the previous instrument and effect, restores their states, and then starts playback. After that, it initializes the GUI and scans the list in the background.
  - If direct loading fails due to old history or file movement, it restores from the scan results after GUI display. Old history is updated once played.
  - If the main sound source is CLAP Floe, when audio processing is rebuilt during loading or tone restoration, the start of the first phrase is delayed by 100ms. Any leading rests in MML are further added. This is a temporary measure for observing symptoms of missing initial sounds, and resolution on actual hardware is unconfirmed. Refer to [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- **Effect Routing**
  - Serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. Connected effects can be removed one by one with `Remove`. Duplicate effects of the same format + ID cannot be connected.
  - The connection order is displayed in `Routing`. Drag the `↕` button of an effect up or down to change its order. While dragging, the moving effect's name and the insertion line are displayed. Instruments are not subject to reordering. You can edit tones and effects using `Show UI` for each plugin. `Instrument` / `Effect` are displayed with the same type color as the list on the left.
  - Removing all effects reverts to a direct connection from the sound source to the output. `Bypass effect` applies to the entire chain, allowing signals to pass through while retaining instance and settings. When bypassed, connected effects in Routing are grayed out, with `Bypassed` appended.
  - `Sequence` is for the instrument. Even if Sequence is stopped, audio processing continues, allowing effect tails to decay.
  - On next launch, the instrument, the order of all effects, their respective states, and Bypass status are restored before playback. History for one old-format effect can also be loaded. If effect restoration fails, an error is displayed, it reverts to a direct sound source connection, and the saved configuration is retained.
  - Add, remove, and reorder operations are saved only if the new connection is successfully prepared. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono to stereo is duplication; stereo to mono is the average of left/right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause brief audio glitches, and sounding notes and their tails may be cut off. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- **Tones**
  - With `★ Add favorite` for each plugin, multiple sound tones/settings can be saved as favorites at that moment. For sound sources, playback patterns, stopped selections, and confirmed MML/chord content are also saved. For effects, playback settings are not saved. Names are assigned automatically and can be changed later.
  - When loading a different plugin, the tone, settings, and playback pattern of the plugin being replaced are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. It is not added on re-loading the same plugin or on restoration at launch. Confirmed MML/chord content and stopped selections for the sound source are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is aborted.
  - During manual and automatic saving, it checks for duplicates of the same plugin, role, playback pattern, confirmed MML content, and state, and reuses saved items and names. For automatic History recording, only differences in stop state are treated as identical, but MML with different content is kept as a separate item. Effects do not compare playback settings.
  - [Plugin-Specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides product-specific exceptions, reasons, scope, and limitations. For confirmed TyrellN6 CLAP state format, the compressed part that changes only with playback is excluded from duplicate detection, and tones with the same text-based tone settings are grouped. Confirmed PCore `UI_op=9/10` and trailing blank lines in text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed Vaporizer2 CLAP format, tiny differences in MSEG time/coordinates (absolute difference for specified items: time < `0.00011` ms, normalized coordinates < `0.000001`. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) caused by loading/saving are treated as identical. Edits within this range cannot be distinguished. The saved data remains as is.
  - Switch lists using the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order, with new additions placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON allows reordering and immediate saving using `↑` / `↓` at the end of the row. Loading by clicking a row and removing Effects works as before. `Sort by plugin name` sorts once by plugin name in ascending order and saves (case-insensitive, same-named items maintain relative order). Manual adjustments are still possible afterward. Reordering is disabled when filtering, and reorder mode is OFF on startup. Refer to [ADR 0017](docs/adr/0017-manual-favorites-order.md) for adoption history and specifications.
  - History is in order of most recent registration. If determined to be the same tone, the saved tone is reused, its registration time is updated, and it is moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are retained. All old-format items are migrated to History, and items not starting with `auto ` are also kept in Favorites (because the old format did not record the origin of automatic saves, this ensures all renamed automatic saves are included in History). Refer to [ADR 0016](docs/adr/0016-history-instead-of-automatic-favorites.md) for policy and reasons.
  - History does not display Rename/Delete menus. Renaming/deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If there are CLAP plugins with the same name, manufacturer, and type, VST3 plugins with matching names/manufacturers are hidden from the Plugins list (leading/trailing spaces and case differences in name/manufacturer are ignored; if name or manufacturer is empty, both are displayed). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and is maintained on next launch. It does not change the format of Favorites or loaded/restored plugins.
  - On both tabs, click the name/image to load, and click again to remove a connected effect. Currently loaded/connected rows are highlighted, and the color is maintained even when an effect is bypassed. The `Load` / `Add` / `Remove` buttons in Plugins perform the same operations.
  - Favorites are displayed one item per line. Long names and additional information are truncated; hover over them to see the full text. Waveforms and spectrums are displayed below Routing on the right side; the width of the right side can be adjusted by dragging the boundary. `On right` in `Audio analysis` switches the analysis display between bottom-right / bottom of screen (default is bottom-right on startup).
  - For confirmed sforzando CLAP state format, it doesn't create new favorites for only differences in save counter and note/CC1 playback values, instead reusing existing favorites. Differences in tone settings are preserved. Refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` is displayed on the left of each row. One click on a favorite name restores the tone/settings. For instrument favorites, it restores the saved confirmed content and playback pattern selection. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites without old-format content retain the current content, and old-format stopped items maintain the current selection. If conversion or state loading/connection fails, the original content/selection/playback state is restored, and the failed content is not saved to the session. Instrument favorites maintain connected effects, and effect favorites maintain the sound source and MML. A sound source is required to call an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed, and re-clicking the name removes that effect. This behavior is the same after editing settings or while bypassed. The sound source, other effects, and saved favorites remain. Another tone of the same format + ID switches settings at the current position, and unconnected effects are appended to the end. The selected favorite for each effect is also restored on next launch.
  - Rename/Delete operations are available from the `…` at the end of the row. Edits after calling or traditional automatic saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Short audio glitches may occur during saving/restoration, and the savable range is the same as traditional state saving.
- **Lissajous Display**
  - `Lissajous` is displayed below the oscilloscope in the right-side analysis pane. If `☰` → `On right` for the oscilloscope is unchecked, the spectrum, oscilloscope, and Lissajous become horizontally arranged at the bottom of the screen.
  - A 45° rotated Lissajous displays the output of the last 50ms (up to 4096 samples), reflecting effects and Bypass. A common automatic magnification preserves volume differences and aspect ratios of shapes; same-level in-phase results in a vertical line, out-of-phase in a horizontal line, and sine waves with phase differences result in ellipses or circles. Output only on the left results in a diagonal line from top-left to bottom-right; output only on the right results in a diagonal line from top-right to bottom-left; mono results in a vertical line; silence results in a central dot.
  - It updates regardless of note, cycle count, or trigger settings, and `Show analysis labels` can display headings. Axes and drawing methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the correlation between left and right channels for the last 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is out-of-phase. Negative values are displayed in red, serving as a guideline for judging cancellation when mixed to mono. For silence or one side silent, it displays "—".
- **Waveform Display**
  - The `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) outputs, reflecting effects and Bypass, overlaid. Mono displays the same waveform on both.
  - The display width is determined from the MIDI note number of the last Note On (A4 = 440Hz), and you can choose `1`, `2`, `4`, `8 cycles`. After Note Off, the decay is displayed based on the last note. Output containing chords or detuning may not perfectly repeat within that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel. If the left channel is silent, it uses the right channel; if there's no crossing, it displays from the beginning.
  - `Similarity` sweeps through the buffer from the beginning, one sample at a time for the display width, to find the position with the maximum correlation coefficient to the previous display. In case of a tie, the earlier position is chosen, and the same start position is used for left and right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when a note is played, the playback pattern changes, the number of display cycles, trigger method, or audio stream changes. It will show a "waiting" display until sufficient data is collected.
  - Exploration is performed on a separate thread from audio processing. Display updates may be slow for low notes or many cycles. If display data is interrupted, the history is reset.
  - Default display settings on startup are 4 cycles and zero-cross. Display settings are not saved.

# Build & Verification Steps
- *Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for steps. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI to install it – it's easy! The overall image is: "easy, just let AI handle it!"*
- *Next prerequisite: If Rust is not installed, ask AI to install it – it's easy!*
- As mentioned, ensure `../uapmd` is built.
- Ensure an audio plugin (instrument) like Surge XT is installed. For example, confirm Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm the window opens.
- Confirm Surge XT and other plugins are listed on the screen.
- Click the `Load` button.
- Confirm it loads and plays sound.

# Various

This text was generated by AI and is hard to read. I plan to fix it later.

## Window Position and Size Settings

You can specify the main window's launch position/size and the plugin GUI's placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved to `status.json` in the same directory, and normal saves do not overwrite the TOML. If a mixed state exists in the old TOML, it will be migrated on first launch, the original file saved as `config.toml.before-status-migration.bak`, and only the status items removed. Comments, formatting, and unknown items in the settings will be preserved, and independent comments attached to deleted items will remain at the end. If an existing `status.json` is present, it will take precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates and are used as the launch position if both are specified. If omitted, it uses the traditional initial position. `width` and `height` are the inner width and height of the main window (logical coordinates). Either can be specified; if omitted or set to 0 or a non-finite value, it reverts to width 900 and height 600. Changes will be reflected on the next launch. The plugin placement method is currently only `right_then_bottom_right`, and the default behavior is the same if omitted. GUI size and placement are calculated based on the actual window frame, screen working area, and DPI. Automatic saving of positions manually moved for the main window or GUI, or per-plugin position specification, are not performed.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this app's directory. TUI's `app` / `patches` and play-server's `core-lib` / `server-config` are always locally referenced from Cargo.toml. Play-server requires `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` needs their re-export. Insufficient checkouts or API will result in build errors, and automatic switching to Git versions is not performed.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The location of UAPMD during build is specified via an environment variable. `config.toml` is not read as build settings.

Do not use `cargo install`; instead, run the `exe` generated directly in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require re-configuration and re-building. Shim DLLs are bundled with the exe and unpacked into a cache directory for loading when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency configuration and operational policy. Local switching of TUI is not needed for building with cat. If building cross-repo with TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; `off` is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the contents of each checkout. When verifying, record the HEAD and diffs of each repo.

Launch the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- *May change depending on mood.*
- Extend the strengths not found in cmrt, such as VST3 and GUI support.
  - Automated GUI operation?
    - Log GUI operations and reproduce quick TUI operations in cmrt? As a hint for plugin-specific customization?
  - Experiment with egui.
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates for diverse functionalities.
  - *This benefits both sides by refining the crates and receiving benefits from their use.*
  - Patch selector
    - Can detect preset patches of CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can be played with a single command specifying patch, effect, arpeggio, etc., from the CLI. Make the same possible (to the extent that merely changing cmrt to cat-plugin-player makes it work).
      - Frequent breaking changes are acceptable. Manual migration is possible later. Better than being paralyzed by fear of change and halting progress.

# Concepts, What We Aim For
- **Musical Instrument**
  - For example, a synthesizer with a MIDI keyboard makes sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations to produce sound.
- **Sound Production**
  - We aim to maintain "it produces sound in the author's environment" as much as possible.
- **Instant Sound**
  - From app launch to sound production, an perceived 0.2 seconds. Playback is prioritized for speed.
    - Processes like GUI display preparation also occur after the sound.
      - Sound playing before the GUI appears might be considered a sound version of a splash screen.
    - Must run the release-built exe directly (via cargo adds timestamp checks, and debug builds add debug code, both causing launch delays).
- **Educational Use**
  - More oriented towards educational use than commercial use.
- **Experimentation, Exploration, Personal Use**
  - Frequent breaking changes.

# Random Thoughts Corner
- *Related to concepts.*
- I love audio plugin presets.
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide a musical instrument UX.
  - Not a DAW UX, but a musical instrument UX.
- What is a musical instrument?
  - It is the synthesis of each layer from audio plugin to PC keyboard.
    - There are intermediate layers.
      - These are plugin hosts and performance UIs.
        - This is a layer that can be "vibe coded."
        - Creating this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - Meaning, we target beginners with a broad range, including cats and babies (e.g., Android tablets).
  - *This is purely a metaphor. Actual operation on Android tablets is unconfirmed. The likelihood of effort to make it work on Android if it doesn't is also low.*

# Out of Scope, Not Aimed For
- **Robustness**
  - Absolutely no bugs, never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- **Features**
  - Equipped with every possible feature. Usable for all purposes.
- **Convenience**
  - Extreme pursuit of convenience and ease of use.
- **Performance**
  - Operates rapidly on all environments without CPU load. Audio never glitches.
- **Requests**
  - Immediate response to all user requests.

# License
- MIT