# cat-plugin-player

A lightweight application that allows you to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Problems Addressed by This Application
- Audio plugin sounds are fantastic!
- To play them, it often requires launching a DAW and extensive setup.
- Lightweight applications for easy playback sometimes require account registration to obtain, or their user experience wasn't suitable for me.
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So, that's what I decided to do.

# Features
- Detection
  - Upon startup, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button automatically starts playback after loading the plugin.
- GUI
  - You can display the plugin's GUI by pressing the `Show UI` button.
- Sound
  - To change the sound, use the plugin's GUI.

※↑This section is written by a human. AI is prohibited from writing here.

# Features (Detailed additions by AI. Intending to refine later as it's hard to read.)

Oscilloscope display methods and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- GUI
  - When you open the GUI for the first time, it waits 2 seconds for drawing, automatically takes a screenshot, and scales it down to a PNG image that matches the aspect ratio of the list display frame and screen zoom level. It saves to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers (ignoring leading/trailing spaces and case) share the same image across VST3 / CLAP. If the name/manufacturer is missing, they are distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated from list/favorite identification information to shared images and used. Sound state and favorite load destinations are still distinguished by format + ID.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while maintaining aspect ratio. Hovering over the image displays it at its saved size. Plugins without a GUI displayed will show a temporary icon of the same height.
  - The GUI prioritizes positioning to fit within the screen's work area, aligned with the right and top edges of the main window. If it doesn't fit, it's placed at the bottom-right of the main window's screen work area. It's positioned on display and GUI resizing, but does not constantly follow main window movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the work area; if its height is greater, its top edge aligns.
  - For blank images or failed captures, it retries at 1-second intervals, up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured, and opening the GUI will replace them with a valid image. After success, it does not recapture.
  - If capture fails even after retrying, an error is displayed. Opening the GUI again will retry. GUIs that do not support capture will remain with a temporary icon. If you want to retake or if the saved PNG is corrupted, delete the shared PNG and any remaining old format-specific PNGs for that plugin, restart the app, and open the GUI (old PNGs are kept during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next launch. The stopped state is also restored.
  - CLAP Note On/Off is sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. The arrows cycle at both ends of the types and do not include "stop". Switching during playback releases all notes in the current phrase and starts from the beginning of the new phrase. When stopped, selecting a type does not play; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except when typing). The selected type and stop state are restored on next launch.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, or `Random` using the same left/right arrows or dropdown. The initial value is 100. Random selects a value from 1 to 127 for each Note On. Changes are reflected from the next note without restarting the phrase and are restored on next launch.
  - Guitar open strings send MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) sequentially at 125ms intervals. 2 seconds after the last Note On, all six notes are Note Offed simultaneously, and the sequence repeats after 0.5 seconds.
  - On successful `Load`, it's memorized when selecting `Sequence` type, starting, or stopping. The memory persists after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, the binary state of sounds, parameters, etc., exposed by the plugin via UAPMD's state API is saved per plugin to `%LOCALAPPDATA%\cat-plugin-player\states\`. On next launch or reloading the same plugin, it is restored before playback begins.
  - `Load` an instrument replaces only the sound source, maintaining connected effects and settings. The state of the plugin being replaced is saved, and it switches only after successful creation, restoration, and connection of the new plugin. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. It does not include duplication of external samples or saving during forced termination.
  - On startup, the previous instrument and effects are loaded directly, states are restored, and playback begins. The GUI is then initialized, and the list is scanned in the background.
  - If direct loading fails due to old history or file movement, it is restored from the scan results after GUI display. Old history is updated once played.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - Serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. Connected effects can be removed one by one with `Remove`. Duplicate connections of effects with the same format + ID are not allowed.
  - `Routing` displays the connection order. You can drag the `↕` button of an effect up or down to change its order. While dragging, the moving effect name and insertion line are displayed. Instruments are not subject to reordering. You can edit sounds or effects using `Show UI` for each plugin.
  - Removing all effects reverts to a direct connection from the sound source to the output. `Bypass effect` targets the entire chain, allowing audio to pass through while retaining instances and settings.
  - `Sequence` is for the instrument. Even if the Sequence is stopped, audio processing continues, allowing effects to trail off.
  - On next launch, the instrument, the order of all effects, their respective states, and Bypass status are restored before playback. History of one old format effect can also be loaded. If an effect fails to restore, an error is displayed, it reverts to a direct connection from the sound source, and the saved configuration is retained.
  - Additions, removals, and reordering are saved only if the new connection setup succeeds. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication; stereo → mono is the average of left and right. Auxiliary input is silent.
  - Changing connections or Bypass may cause a brief audio dropout, and sounding notes and decay may be interrupted. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Sound
  - You can save multiple instances of the current sound, settings, and playback pattern (including stopped state) as favorites using `★ Add favorite` for each plugin. Names are assigned automatically and can be changed later.
  - Loading a different plugin automatically records the sound, settings, and playback pattern of the plugin being replaced into History before switching. The name is `Plugin Name Sequence Number`. It is not added on reloading the same plugin or restoring on startup. If saving fails, the switch is aborted.
  - During manual and automatic saving, it checks for duplicates of the same plugin, role, playback pattern, and state, keeping the newer favorite and deleting older items and saved files.
  - [Plugin-Specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides exceptions, reasons, scope, and limitations for each product. For TyrellN6 CLAP's verified state format, the compressed part that changes only with playback is excluded from duplicate detection, and sounds with identical text sound settings are grouped. Verified PCore `UI_op=9/10` and blank lines at the end of text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For Vaporizer2 CLAP's verified format, minute differences in MSEG time/coordinates that occur during loading/saving (absolute difference of specified items: time `0.00011` ms or less, normalized coordinate `0.000001` or less. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - Switch lists using the `Favorites` / `History` / `Plugins` tabs in the center. Favorites maintain their manual order, with new additions placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON allows sorting by `↑` / `↓` at the end of the row, saving immediately. Loading by clicking a row and removing Effects works as before. `Sort by plugin name` sorts once by plugin name in ascending order and saves (case-insensitive, maintaining relative order for items with the same name). Manual adjustments can be made afterward. Sorting is disabled when filtering, and reorder mode is OFF on startup. See [ADR 0017](docs/adr/0017-manual-favorites-order.md) for adoption history and specifications.
  - History is sorted by most recent registration. If the same sound is detected, the existing sound is reused, its registration time is updated, and it is moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are preserved. All items of the old format are migrated to History, and all items except those starting with `auto ` are also kept in Favorites (because the old format does not record the origin of auto-saves, thus including all renamed auto-saves in History). Refer to [ADR 0016](docs/adr/0016-history-instead-of-automatic-favorites.md) for policy and reasons.
  - History does not display Rename/Delete operation menus. Renaming and deletion are performed in Favorites.
  - The `Show only CLAP for duplicate plugins` option in the global `☰` settings is ON by default. If there are CLAP plugins with the same name, manufacturer, and type, VST3 plugins in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are displayed). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on next launch. It does not change the format of Favorites or loaded/restored plugins.
  - In both tabs, you can load by clicking the name/image, and disconnect connected effects by clicking again. The currently loaded/connected row is highlighted, and the color is maintained even when effects are bypassed. The same operations can be performed with the `Load` / `Add` / `Remove` buttons in Plugins.
  - Favorites are displayed one item per line. Long names and supplementary information are truncated; hovering the mouse over them reveals the full text. Waveform/spectrum are displayed below Routing on the right side, and the width of the right side can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles the analysis display between bottom-right and bottom of the screen (defaulting to bottom-right on startup).
  - For sforzando CLAP's verified state format, it does not increase the number of identical sounds based solely on differences in the save counter and played values of notes/CC1, instead reusing existing favorites. Differences in sound settings are retained. Refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - Colored `Instrument` / `Effect` is displayed to the left of each row. Single-clicking a favorite name restores the sound, settings, and playback pattern. Favorites saved in a stopped state are restored as stopped. Instrument favorites retain connected effects, and effect favorites retain the instrument. An instrument is required to call an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed, and re-clicking the name removes that effect. This behavior is the same after editing settings or during Bypass. The instrument, other effects, and saved favorites remain. Another sound with the same format + ID switches settings at the current position, and unconnected effects are appended to the end. The selected favorite for each effect is also restored on next launch.
  - Rename/Delete operations are available from the `…` at the end of the row. Edits after calling a favorite or traditional auto-saves do not overwrite the favorite. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. There may be brief audio dropouts during saving/restoration, and the savable range is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right-side analysis pane. If `☰` → `On right` is unchecked for the oscilloscope, the spectrum, oscilloscope, and Lissajous appear side-by-side at the bottom of the screen.
  - A 45° rotated Lissajous displays the most recent 50ms of output (max 4096 samples), reflecting effects and Bypass. A common automatic magnification maintains volume differences and aspect ratio of the figure; in-phase at the same level becomes a vertical line, out-of-phase a horizontal line, and sine waves with phase differences become ellipses or circles. Output only on the left is a diagonal line from top-left to bottom-right, only on the right is top-right to bottom-left, mono is a vertical line, and silence is a central dot.
  - It updates independently of notes, cycle counts, or trigger settings, and `Show analysis labels` can display headings. Axes and drawing methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation for the most recent 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is out-of-phase. Negative values are displayed in red, serving as a guideline for judging cancellation when mixed to mono. For silence or one side being silent, it shows "—".
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) channels of the output, reflecting effects and Bypass, overlaid. Mono displays the same waveform for both.
  - The display width is determined from the MIDI note number of the last Note On (A4 = 440Hz), and `1`, `2`, `4`, `8 cycles` can be selected. After Note Off, the decay is still displayed based on the last note. Output containing chords or detuning may not repeat perfectly within that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel. If the left channel is silent, it uses the right channel; if there's no crossing, it displays from the beginning.
  - `Similarity` sweeps from the start of the buffer, sample by sample, for the display width, to find the position with the maximum correlation coefficient to the previous display. Ties prefer the earlier position, and the same start position is used for both left and right. For the first display or if the comparison target is silent, zero-crossing is used.
  - Comparison history is reset when a note is sounded, playback pattern changes, display cycle count changes, trigger method changes, or the audio stream changes. It displays "waiting" until sufficient data is collected.
  - Exploration is performed on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - Default display settings on startup are 4 cycles and zero-cross. Display settings are not saved.

# Build & Verification Steps
- ※Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Please refer to its instructions. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI to install it, and it's easy! The overall image is "easy by entrusting it to AI!"
- ※Next prerequisite: If Rust is not installed, ask AI to install it, and it's easy! That's the image.
- As mentioned, confirm that `../uapmd` is already built.
- Confirm that an audio plugin (instrument) like Surge XT is installed. For example, confirm Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the screen opens.
- Confirm that Surge XT and similar plugins are listed on the screen.
- Click the Load button.
- Confirm that it loads and plays sound.

# Various Things

The AI-generated text is hard to read. I plan to fix it later.

## Window Position and Size Settings

You can specify the main window's startup position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the application. Session and favorite selection states are saved in `status.json` in the same directory, and regular saves do not overwrite the TOML. If status items are mixed in the old TOML, they will be migrated on first launch, the original file saved as `config.toml.before-status-migration.bak`, and only status items removed. Configuration comments, format, and unknown items are preserved, and independent comments attached to deleted items are moved to the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates, used as the startup position if both are specified. If omitted, it uses the traditional initial position. `width` and `height` are the inner width/height (logical coordinates) of the main window. Either can be specified alone; omitted values or values <= 0 / non-finite values revert to a width of 900 and height of 600. Changes are reflected on next launch. The plugin placement method is currently only `right_then_bottom_right`; omitting it results in the same behavior. GUI size and placement are calculated based on the actual window frame, screen work area, and DPI. Automatic saving of manually moved main window/GUI positions and per-plugin position specification are not implemented.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then run from this app's directory. TUI's `app` / `patches` and play-server's `core-lib` / `server-config` are always referenced locally from Cargo.toml. Play-server requires `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` requires their re-export. Insufficient checkout or API will result in a build error; automatic switching to Git versions is not performed.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The UAPMD location during build is specified via environment variable. `config.toml` is not read as a build setting.

Do not use `cargo install`; directly execute the `exe` generated in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require re-configuration/re-build. Shim DLLs are bundled with the exe and extracted to a cache directory for loading when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency structure and operational policy. Local switching of TUI is not required for building cat. If building across repos with TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; 'off' is a human task. Local references use uncommitted changes, and Cargo.lock does not fix the contents of each checkout. When verifying, record the HEAD and differences of each repo.

Launch the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- ※Subject to change based on mood
- Enhance strengths not present in cmrt, such as VST3 and GUI support
  - Automated GUI operation?
    - Log GUI operations and reproduce quick TUI operations in cmrt? Provide hints for plugin-specific customization?
  - Experiment with egui
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates to achieve various goals
  - ※This will also refine the crates, and we benefit from using them, so it's a win-win.
  - Patch selector
    - Detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can play sounds in one go from the CLI, specifying patches, effects, arpeggios, etc. Make the same possible (to the extent of being able to play just by changing cmrt to cat-plugin-player).
      - Frequent destructive changes are acceptable. Manual migration is possible later. Better than being paralyzed by fear of change and halting progress.

# Concept, What We Aim For
- Musical Instrument
  - For example, a synthesizer with a MIDI keyboard makes sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations until sound is produced.
- Sounding
  - We aim to maintain "it plays in the author's environment" as much as possible.
- Immediate Sound
  - From app launch to sound production, an perceived 0.2 seconds. Playback is optimized for speed.
    - Processes like GUI display preparation are performed after the sound starts.
      - Sound playing before GUI display might be considered the audio equivalent of a splash screen.
    - A release-built exe must be run directly (cargo adds timestamp checks, debug builds add debug code, both cause startup delay).
- Educational
  - It's oriented more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be made frequently.

# Random Thoughts Corner
- ※Related to the concept
- I love audio plugin presets.
  - Because they are reproducible.
  - Because they are shared freely, benefiting everyone.
- I want to provide a musical instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is a musical instrument?
  - It's a combination of each layer from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and playback UI.
        - This is a layer where "vibe coding" can be done.
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - This means extending the target range to beginners, even including cats and babies (e.g., Android tablets) as a metaphor.
  - ※This is merely a metaphor. Whether it actually runs on an Android tablet is unconfirmed. The likelihood of striving to make it work on Android if it doesn't is also low.

# Out of Scope, What We Don't Aim For
- Robustness
  - Absolutely no bugs, the app never crashes.
  - Full compatibility. Loads all past settings and data and operates perfectly.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed with no CPU load in all environments. Audio never drops out.
- Requests
  - Immediate response to all user requests.

# License
- MIT