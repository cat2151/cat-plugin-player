# cat-plugin-player

A lightweight application for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- Audio plugins offer fantastic sounds!
- However, playing them often requires launching a DAW and performing setup tasks.
- Lightweight apps for casual playing sometimes required account registration to obtain, or their user experience wasn't to my liking.
- In modern times, if you have a desired UX, you can achieve it through "vibe coding".
- So, I decided to do just that.

# Features
- Discovery
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button automatically starts playback after the plugin loads.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI.
- Preset
  - To change the preset, use the plugin's GUI.

※↑This section is written by a human. AI is prohibited from writing here.

# Features (Detailed additions by AI. Intended to be refined later due to readability issues)

The display method and constraints of the oscilloscope are documented in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- GUI
  - When opening a GUI for the first time, it waits 2 seconds for rendering, then automatically takes a screenshot, scales it down to a PNG size that fits the list display frame and screen magnification while maintaining aspect ratio. It is saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers share the same image for VST3 / CLAP (ignoring leading/trailing spaces and case). If the name or manufacturer is missing, it distinguishes by format + ID.
  - Previously saved format-specific images are also automatically migrated to shared images and used based on list/favorite identification information. Preset state and favorite load destinations are still distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while maintaining aspect ratio. Hovering over an image displays it at its saved size. Plugins without a GUI displayed will show a temporary icon of the same height.
  - The GUI prioritizes placement to the right of the main window, aligned with its top edge, fitting within the screen's work area. If it doesn't fit, it's placed at the bottom-right of the main window's screen work area. This placement occurs when displayed and when the GUI is resized, but it does not continuously follow the main window's movement. The GUI is not scaled down; if its width is larger than the screen, its left edge aligns with the work area; if its height is larger, its top edge aligns.
  - For blank images or failed captures, retries occur at 1-second intervals for up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured and replaced with a valid image when the GUI is opened. No re-captures occur after success.
  - If capture fails after retries, an error is displayed. Reopening the GUI retries. For GUIs that do not support capture, a temporary icon remains. If you wish to retake or if a saved PNG is corrupted, delete the shared PNG and any remaining old format-specific PNGs for that plugin, restart the application, and open the GUI (old PNGs are retained during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on the next launch. Stop state is also restored. The save scope for MML/chord input phrases is as described below.
  - CLAP Note On/Off is sent according to the input format supported by the plugin. This includes sforzando CLAP, which only accepts MIDI. The scope of fixes and validation records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. Arrows cycle through the ends of the types; stop is not included in the types. Switching during playback releases all sounding notes of the current phrase and starts from the beginning of the new phrase. When stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except during text input). The selection of normal playback patterns and the stop state are also restored on the next launch.
  - To audition MML/chord progressions, open the input screen with the `i` key or the `MML / Chord (i)` button in `Sequence`. A single note example is `t120 c8r8d4e4r4`; a chord progression example is `C F G C`. The same input field accepts both, attempting chord notation conversion first. Uppercase `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter` / `Confirm` and canceled with `Esc` / `Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are used for text input, and `Space` for play/stop does not function.
  - Confirmed phrases loop with `Play` / `Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Confirming while stopped maintains the stop state; confirming while playing restarts from the beginning of the new phrase. You can also switch to normal playback patterns. Input content is retained only during the current session and is not saved to startup restoration, favorites, or History. If an MML/chord input pattern is selected and the application is closed, the next launch will default to the 4-note pattern, and the stop state will be maintained.
  - MML note velocity prioritizes the `Velocity` field setting over any in-input specification. CC1 can also be controlled from the `CC1 modulation` field. If a very dense phrase reaches the playback event capacity, sounding notes are released, and playback stops. It can be restarted with `Stop` -> `Play`.
  - When recalling from the app's Favorites/History or changing presets via Random patch, the stop/playback state is maintained. During playback, it starts from the beginning of the phrase and does not inherit any mid-phrase waits from the previous phrase. Leading rests specified in MML are preserved. Preset changes made via the plugin's own GUI are not detected.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification raises values in Note On order within the phrase and then lowers them in the next loop, repeating the action. Changes are applied from the next note without restarting the phrase and are restored on the next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or the dropdown. The initial value is 0. `sweep` repeatedly changes from 0 to 127 over 2 seconds, then from 127 to 0 over the next 2 seconds. The selection is also restored on the next launch.
  - Open guitar strings (E, A, D, G, B, E) trigger MIDI note numbers 40, 45, 50, 55, 59, 64 sequentially with 125ms intervals for Note On. Two seconds after the last Note On, all six notes are simultaneously Note Off, repeating after 0.5 seconds.
  - On successful `Load`, it is remembered during `Sequence` type selection, start, and stop. The memory persists after stopping or `Remove`. Older history/favorites are restored with the conventional 4-note pattern.
  - On exit and `Remove`, the binary state of presets/parameters exposed by plugins via UAPMD's state API is saved to `%LOCALAPPDATA%\cat-plugin-player\states\` per plugin. This state is restored before playback begins on the next launch or when the same plugin is reloaded.
  - `Load` for an instrument only replaces the sound source, maintaining connected effects and settings. The state of the plugin being replaced is saved, and switching occurs only after successful creation, restoration, and connection of the new plugin. If it fails, the original configuration is retained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but UAPMD's current VST3 implementation only retrieves component state. It does not include external sample replication or saving on forced termination.
  - On launch, the previous instrument and effect are loaded directly, and their state is restored before playback begins. The GUI is then initialized, and the plugin list is scanned in the background.
  - If direct loading is not possible due to old history or file movement, it is restored from the scan results after the GUI is displayed. Old history is updated once played.
  - If the main sound source is CLAP Floe, when audio processing is rebuilt during loading or preset restoration, the start of the first phrase is delayed by 100ms. Leading rests in MML are further added. This is a temporary measure for observing the symptom of missing initial notes, and a fix on actual hardware is unconfirmed. Refer to [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - Routing is a serial connection: 1 instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects to the end. Connected effects can be removed one by one with `Remove`. Duplicate connections of effects with the same format + ID are not allowed.
  - The `Routing` section displays the connection order. You can reorder effects by dragging the `↕` button up or down. While dragging, the moving effect's name and an insertion line are displayed. Instruments are not subject to reordering. You can edit presets or effects using `Show UI` for each plugin. `Instrument` / `Effect` are displayed with the same type color as in the left list.
  - Removing all effects reverts to a direct connection from the sound source to the output. `Bypass effect` targets the entire chain, allowing audio to pass through while retaining the instance and settings. When bypassed, connected effects in Routing are displayed in grey, with `Bypassed` appended.
  - `Sequence` is for instruments. Even if `Sequence` is stopped, audio processing continues to allow effects to decay.
  - On the next launch, the instrument, the order of all effects, their respective states, and the Bypass state are restored before playback. A single effect history from an old format can also be loaded. If effect restoration fails, an error is displayed, the sound source reverts to a direct connection, and the saved configuration is retained.
  - Additions, removals, and reordering are saved only if the new connection setup succeeds. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication; stereo → mono is an average of left and right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause a short audio dropout, and sounding notes and their decay may be cut off. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Preset
  - With `★ Add favorite` for each plugin, you can save multiple presets/settings/playback patterns (including stop state) as favorites at that moment. MML/chord input phrases are not saved; if playing, it records as a 4-note pattern; if stopped, as stopped. Names are generated automatically and can be changed later.
  - When `Load`ing a different plugin, the preset/settings/playback pattern of the replaced plugin are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. No additions are made for reloading the same plugin or restoring on startup. Recording playback patterns during MML/chord input is treated the same as favorites. If saving fails, the switch is aborted.
  - During manual and automatic saving, it checks for duplicates of the same plugin, role, playback pattern, and state, retaining the newer favorite and deleting older entries and save files.
  - In [Plugin-Specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md), you can check exceptions, reasons, scope, and limitations for each product. For TyrellN6 CLAP's confirmed state format, compressed parts that change only with playback are excluded from duplicate judgment, and presets with identical text-based tone settings are grouped. Confirmed PCore `UI_op=9/10` and blank lines at the end of the text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For Vaporizer2 CLAP's confirmed format, minute differences in MSEG time/coordinates arising from loading/saving (absolute difference for specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less; details in [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md)) are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - Switch lists using the `Favorites` / `History` / `Plugins` tabs in the center. Favorites maintain manual order, with new additions placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - Turning on `Reorder favorites` in the Favorites hamburger menu allows you to reorder using `↑` / `↓` at the end of the row and saves immediately. Loading by clicking a row and removing Effects works as before. `Sort by plugin name` sorts and saves items once in ascending order of plugin name (case-insensitive, preserving relative order for same-named items). Manual adjustments are possible afterward. Sorting is disabled during filtering, and sort mode is OFF on startup. Refer to [Background and Specifications (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is ordered by most recent registration. If the same preset is detected, the existing preset is reused, its registration time is updated, and it's moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are retained. All old format items are migrated to History, and items not starting with `auto ` are also kept in Favorites (this is because the old format did not record the origin of automatic saves, so all renamed automatic saves are included in History). Refer to [Policy and Rationale (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus. Renaming/deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If there's a CLAP plugin with the same name, manufacturer, and type, VST3s with matching names/manufacturers are hidden from the Plugins list (ignoring leading/trailing spaces and case differences; if name or manufacturer is empty, both are displayed). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on the next launch. It does not change the format of Favorites or loaded/restored plugins.
  - On both tabs, clicking the name/image loads it, and for connected effects, clicking again disconnects them. The currently loaded/connected row is highlighted, and the color is maintained even when effects are bypassed. The same operations can be performed with the `Load` / `Add` / `Remove` buttons in Plugins.
  - Favorites are displayed one item per line. Long names and supplementary information are abbreviated; hover the mouse to see the full text. Waveform/spectrum are displayed below Routing on the right side, and the width of the right side can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles the analysis display between bottom-right and bottom of the screen (default is bottom-right on startup).
  - In sforzando CLAP's confirmed state format, existing favorites are reused without creating new ones solely based on differences in save counter and played note/CC1 values. Differences in preset settings are retained. Refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - Colored `Instrument` / `Effect` are displayed to the left of each row. A single click on a favorite name restores its preset/settings. If stopped, it remains stopped; if MML/chord is playing, the current phrase is maintained. Calling an instrument favorite during normal pattern playback switches to the saved playback pattern (MML/chord playback saves as a 4-note pattern). If an item saved while stopped is called, the current playback pattern is maintained. Instrument favorites retain connected effects; effect favorites retain the instrument. An instrument is required to call an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed, and clicking the name again removes that effect. This behavior is the same after editing settings or during Bypass. The instrument, other effects, and saved favorites remain. A different preset with the same format + ID switches settings at the current position, and unconnected effects are appended to the end. The selected favorite for each effect is also restored on the next launch.
  - Rename/Delete can be performed from `…` at the end of the row. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last favorite saved or called.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Saving and restoring may cause short audio dropouts, and the save scope is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right-hand analysis pane. If `☰` → `On right` is unchecked in the oscilloscope, the spectrum, oscilloscope, and Lissajous display horizontally at the bottom of the screen.
  - A 45° rotated Lissajous curve displays the last 50ms of output, reflecting effects and Bypass (max 4096 samples). A common automatic gain keeps volume differences and aspect ratios of figures; same-level in-phase results in a vertical line, out-of-phase a horizontal line, and sine waves with phase differences become ellipses or circles. Left-only output is a diagonal line from top-left to bottom-right; right-only is top-right to bottom-left; mono is a vertical line; silence is a center dot.
  - It updates independently of notes, cycle counts, or trigger settings, and `Show analysis labels` can display headings. Axis and rendering methods can also be checked in graph tooltips.
  - Immediately below Lissajous, `Correlation` displays the left/right correlation over the last 150ms as a bar and numerical value from -1 to +1. +1 indicates in-phase, near 0 indicates weak correlation, and -1 indicates out-of-phase. Negative values are displayed in red, serving as a guideline for judging cancellation when mixed to mono. If silent or one side is silent, it displays '—'.
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) outputs, reflecting effects and Bypass, superimposed. Mono displays the same waveform on both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and `1`, `2`, `4`, `8 cycles` can be selected. After Note Off, the decay is displayed based on the last note. Output containing chords or detuning may not perfectly repeat within that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel. If the left channel is silent, the right channel is used; if no crossing, it displays from the beginning.
  - `Similarity` exhaustively searches for the position with the maximum correlation coefficient with the previous display, sweeping from the buffer start for the display width, 1 sample at a time. In case of a tie, the earlier position is adopted, and the same start position is used for both left and right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when the note playback, playback pattern, display cycle count, trigger method, or audio stream changes. A waiting display is shown until sufficient data is gathered.
  - The search is performed in a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - The display settings on startup are 4 cycles and zero cross. Display settings are not saved.

# Build & Verification Steps
- ※Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for steps. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask an AI how to install it, and it'll be easy! The overall impression is that it's easy to handle with AI!
- ※Next prerequisite: If Rust is not installed, ask an AI how to install it, and it'll be easy! The overall impression is that it's easy to handle with AI!
- As mentioned, ensure that ../uapmd has been built.
- Ensure that audio plugins (instruments) like Surge XT are installed. For example, verify that Surge XT produces sound in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the screen opens.
- Confirm that Surge XT and similar plugins are listed on the screen.
- Click the Load button.
- Confirm that it loads and produces sound.

# Miscellaneous

This text was generated by AI and is hard to read. I plan to refine it later.

## Window Position and Size Settings

You can specify the main window's launch position/size and the plugin GUI's placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Edit it after exiting the application. Session and favorite selection states are saved to `status.json` in the same directory, and regular saves do not overwrite the TOML. Mixed states in old TOML are migrated on first launch: the original file is saved as `config.toml.before-status-migration.bak`, then only state items are removed. Setting comments, formatting, and unknown items are preserved, and independent comments attached to removed items remain at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates, used as the launch position if both are specified. If omitted, it defaults to the conventional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. You can specify just one; omitted values or values ≤ 0 or non-finite revert to width 900 and height 600. Changes are reflected on the next launch. The plugin placement method is currently only `right_then_bottom_right`; omitting it results in the same behavior. GUI size and placement are calculated based on the actual window frame, screen work area, and DPI. Automatic saving of manually moved window/GUI positions and per-plugin position specification are not performed.

## Local Build and Execution

Clone this application, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this application's directory. The TUI's `app` / `patches` and play-server's `core-lib` / `server-config` are always locally referenced from Cargo.toml. `prepare_clap_patch_state` / `PatchStateError` are required for play-server, and its re-export is needed for the TUI's `cmrt-patches`. Insufficient checkouts or missing APIs will result in build errors; no automatic switching to Git versions will occur.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in a sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The location of UAPMD during build is specified by an environment variable. `config.toml` is not read as a build configuration.

Do not use `cargo install`; directly execute the `exe` generated in the default `target/release`. The build uses local code (including uncommitted changes) and the existing `target/shim`, but source changes may require re-configuration and re-building. Shim DLLs are bundled with the exe and unpacked to a cache directory for loading when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the dependency configuration and operational policy of the three repositories. Local TUI switching is not required for building cat. If cross-repository building with TUI itself, use the TUI's existing `python scripts/cross_repo_local.py on` procedure, with humans responsible for `off`. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. During verification, record the HEAD and diffs of each repository.

Launch the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- ※Subject to change based on mood
- Extend the strengths not present in cmrt, such as VST3 and GUI support.
  - Automated GUI operation?
    - Log GUI operations, then reproduce snappy operations with TUI in cmrt? As a hint for plugin-specific customization?
  - Play around with egui
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates to do various things
  - ※This also refines the crates, and we benefit from using them, so it's a win-win.
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can play a patch, effect, arpeggio, etc., with a single command from the CLI. Make the same possible (to the level where changing `cmrt` to `cat-plugin-player` is enough to play).
      - Frequent breaking changes are acceptable. Manual migration is possible later. Better than progress stalling due to fear of change.

# Concept, What We Aim For
- Musical Instrument
  - For example, a synthesizer with a MIDI keyboard produces sound simply by powering it on and pressing a key.
    - We aim for that level of ease of use.
      - Minimize operations required to produce sound.
- Sound Production
  - We aim to maintain "sound production in the author's environment" as much as possible.
- Immediate Sound
  - Perceptually 0.2 seconds from app launch to sound. Playback is made as fast as possible.
    - Processes like GUI display preparation are performed after sound.
      - Sound playing before the GUI appears might be considered a "sound splash screen."
    - You need to run the release-built exe directly (running via cargo has timestamp checks, and debug builds have debug code, both adding to startup latency).
- Educational Use
  - It's geared more towards educational use than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

# Random Thoughts Corner
- ※Related to the concept
- I love audio plugin presets.
  - Because they are reproducible.
  - Because they are freely shared, everyone can benefit.
- I want to provide a musical instrument UX.
  - Not a DAW UX, but a musical instrument UX.
- What is a musical instrument?
  - It is the sum of each layer from audio plugins to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and performance UI.
        - This is a layer that can be "vibe coded".
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies
    - That is, expanding the target range towards beginners to include even cats and babies (e.g., on an Android tablet).
  - ※This is merely a metaphor. Whether it actually runs on an Android tablet is unconfirmed. The likelihood of effort being made to make it work on Android if it doesn't is also low.

# Out of Scope, What We Don't Aim For
- Robustness
  - Absolutely no bugs, no crashes, no matter what.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation with no CPU load in any environment. Audio never drops out.
- User Requests
  - Immediately responding to all user requests.

# License
- MIT