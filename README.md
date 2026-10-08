# cat-plugin-player

A lightweight app for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- Audio plugin tones are wonderful!
- To play them, it often requires launching a DAW and performing setup tasks.
- Lightweight apps for easy playback sometimes require account registration to obtain, or their operation feels "not for me."
- Nowadays, if you have a desired UX, you can achieve it through vibecoding.
- So, that's what I decided to do.

# Features
- Discovery
  - Automatically detects installed plugins upon startup
- Playback
  - After pressing the `Load` button, the plugin loads and playback automatically starts.
- GUI
  - You can display the plugin's GUI by pressing the `Show UI` button.
- Tones
  - To change tones, use the plugin's GUI.

※↑This section is human-written. AI is prohibited from writing here.

# Features (Detailed additions by AI. Hard to read, so I'll fix it later)

The oscilloscope display method and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- GUI
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG with its aspect ratio preserved, fitting the list's display frame and screen zoom level. It saves to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`. Plugins with matching names and manufacturers share the same image across VST3 / CLAP (ignoring leading/trailing spaces and case). If the name or manufacturer is missing, it distinguishes by format + ID.
  - Previously saved format-specific images are also automatically migrated and used as shared images based on the list/favorites identification. Tone states and favorite load destinations continue to be distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height, preserving their aspect ratio. Hovering over an image displays it at its saved size. Plugins without a displayed GUI show a temporary icon of the same height.
  - The GUI prioritizes positions that fit within the screen's work area, right next to and aligned with the top of the main window. If it doesn't fit, it's placed in the bottom-right of the work area of the screen where the main window is located. It is positioned on display and when the GUI is resized, but it does not constantly follow the main window's movement. The GUI is not scaled down; if its width is larger than the screen, its left edge aligns with the work area; if its height is larger, its top edge aligns.
  - For blank images or failed captures, it retries at 1-second intervals for up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured; opening the GUI will replace them with valid images. After a successful capture, it does not re-capture.
  - If capturing fails after retries, an error is displayed. Opening the GUI again will retry. GUIs that don't support capture will remain with a temporary icon. If you want to retake or if the saved PNG is corrupted, delete the shared PNG and any remaining old format-specific PNGs for that plugin, restart the app, and open the GUI (old PNGs are kept during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on the next startup. The stopped state is also restored.
  - CLAP Note On/Off messages are sent according to the input format supported by the plugin. This includes sforzando CLAP, which only accepts MIDI. The scope of fixes and validation records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using the left/right arrows or the dropdown. The arrows cycle at the ends of the types; Stop is not included. Switching during playback releases all sounding notes in the current phrase and starts the new phrase from the beginning. When stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except when typing). The selected type and stopped state are restored on the next startup.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, or `Random` using the same left/right arrows or dropdown. The initial value is 100. Random selects 1-127 for each Note On. Changes take effect from the next note without restarting the phrase and are restored on the next startup.
  - Guitar open strings send Note On for MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) at 125ms intervals. 2 seconds after the last Note On, all 6 notes are simultaneously Note Off, and the sequence repeats after 0.5 seconds.
  - Saved on `Load` success, and when selecting type, starting, or stopping in `Sequence`. The memory persists after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, the binary state (tones, parameters, etc.) exposed by the plugin via UAPMD's state API is saved to `%LOCALAPPDATA%\cat-plugin-player\states\` per plugin. On the next startup or when the same plugin is re-loaded, it's restored before playback starts.
  - `Load` for an instrument replaces only the sound source, maintaining connected effects and settings. It saves the state of the plugin being replaced, and only switches after successfully creating, restoring, and connecting the new plugin. If it fails, it maintains the original configuration and displays an error.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. It requests UI state, but the current UAPMD VST3 implementation only retrieves component state. It does not include replication of external samples or saving during forced termination.
  - On startup, it directly loads the previous instrument and effects, restores their states, and then starts playback. After that, it initializes the GUI and scans the plugin list in the background.
  - If direct loading fails due to old history or file movement, it restores from the scan results after GUI display. Old history is updated once played.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - A serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. `Remove` a connected effect to disconnect only that one. Duplicate connections of effects with the same format + ID are not allowed.
  - The connection order is displayed in `Routing`. You can change the order by dragging the effect's `↕` button up or down. While dragging, the moving effect's name and an insertion line are displayed. Instruments are excluded from reordering. You can edit tones and effects using each plugin's `Show UI`.
  - Removing all effects reverts to a direct connection from the instrument to the output. `Bypass effect` targets the entire chain, allowing signals to pass through while retaining instances and settings.
  - `Sequence` is for the instrument. Stopping the sequence continues audio processing, allowing effect decay.
  - On the next startup, the instrument, the order of all effects, their respective states, and Bypass status are restored before playback. History of a single old-format effect can also be loaded. If an effect fails to restore, an error is displayed, the instrument reverts to a direct connection, and the saved configuration is retained.
  - Additions, removals, and reordering are saved only if the preparation for the new connection succeeds. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplicated, stereo → mono is the average of left and right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause short audio dropouts, and sounding notes and decay may be interrupted. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Tones
  - With `★ Add favorite` for each plugin, you can save multiple presets of the current tone, settings, and playback pattern (including stop) as favorites. Names are automatically assigned and can be changed later.
  - When you `Load` a different plugin, the tone, settings, and playback pattern of the plugin being replaced are automatically saved to Favorites before the switch. The name is `auto plugin_name serial_number`. This is not added when reloading the same plugin or restoring on startup. If saving fails, the switch is aborted.
  - During manual and automatic saving, it checks for duplicates of the same plugin, role, playback pattern, and state, keeping the new favorite and deleting older entries and saved files.
  - In [Plugin-Specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md), you can review product-specific exceptions, reasons, scope, and limitations. For confirmed TyrellN6 CLAP state format, compressed parts that change only with playback are excluded from duplicate detection, and tones with identical text tone settings are grouped. Confirmed PCore `UI_op=9/10` and trailing empty lines in text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed Vaporizer2 CLAP format, tiny differences in MSEG time and coordinates (absolute difference of specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less; see [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) caused by loading/saving are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - The central `Favorites` / `Plugins` tabs toggle the list view. Favorites are displayed from newest to oldest and can be filtered by favorite name, plugin name, and format.
  - `Show only CLAP for duplicate plugins` in the `☰` global settings is ON by default. If a CLAP with the same name, manufacturer, and type exists, VST3s with identical name/manufacturer are hidden from the Plugins list (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are shown). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and persists on the next startup. It does not change the format of Favorites or what is loaded/restored.
  - In both tabs, clicking on a name/image loads it; for connected effects, clicking again removes it. The currently loaded/connected row is highlighted, and the color persists even when effects are bypassed. The `Load` / `Add` / `Remove` buttons in Plugins perform the same operations.
  - Favorites are displayed one per line. Long names and supplementary information are truncated; hovering over them reveals the full text. Waveform/spectrum is displayed below Routing on the right; the width of the right panel can be adjusted by dragging the border. `Audio analysis`'s `On right` toggles the analysis display between bottom-right and bottom of the screen (it's bottom-right on startup).
  - For confirmed sforzando CLAP state format, it reuses existing favorites instead of creating new ones for differences only in the save counter and played values of notes/CC1. Differences in tone settings are preserved. Refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` label is displayed to the left of each row. A single click on a favorite name restores its tone, settings, and playback pattern. Favorites saved in a stopped state are restored as stopped. Instrument favorites retain connected effects, and effect favorites retain the instrument. An instrument is required to call an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed; re-clicking the name removes that effect. This behavior is the same after editing settings or during Bypass. The instrument, other effects, and saved favorites remain. A different tone of the same format + ID switches settings at the current position, and an unconnected effect is added to the end. The selected favorite for each effect is also restored on the next startup.
  - From the `…` at the end of the line, you can rename or delete. Edits after recalling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved or recalled favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Saving and restoring may cause short audio dropouts, and the scope of what can be saved is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right-hand analysis pane. If `☰` → `On right` is unchecked for the oscilloscope, the spectrum, oscilloscope, and Lissajous become horizontally aligned at the bottom of the screen.
  - It draws the recent 50ms output (up to 4096 samples) reflecting effects and Bypass, within a square with L on the horizontal axis and R on the vertical axis. A shared automatic gain maintains volume differences; in-phase signals are a diagonal line rising to the right, out-of-phase signals are a diagonal line falling to the right, and sine waves with phase differences form ellipses or circles. Mono signals result in an in-phase diagonal line, and silence results in a central dot.
  - It updates independently of notes, cycle counts, or trigger settings, and `Show analysis labels` can display headings. Axes and drawing methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation for the recent 150ms as a bar and numerical value from -1 to +1. +1 indicates in-phase, near 0 indicates weak correlation, and -1 indicates out-of-phase. Negative values are displayed in red and serve as a guide for judging cancellation when mixed to mono. It shows "—" for silence or if one side is silent.
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen displays L (green) and R (blue) of the output, reflecting effects and Bypass, overlaid. Mono displays the same waveform for both.
  - The display width is determined from the MIDI note number of the last Note On (A4 = 440Hz), and you can choose `1`, `2`, `4`, `8 cycles`. After Note Off, the decay is displayed based on the last note. Outputs including chords or detuning may not perfectly repeat within that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel. If the left channel is silent, it uses the right channel; if there's no crossing, it displays from the beginning.
  - `Similarity` exhaustively searches from the beginning of the buffer for the display width, one sample at a time, for the position where the correlation coefficient with the previous display is maximized. Ties favor the earlier position, and the same start position is used for both left and right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when a note is sounded, playback pattern changes, display cycle count changes, trigger method changes, or the audio stream changes. It displays "waiting" until sufficient data is collected.
  - The search runs on a separate thread from audio processing. Display updates may be slower for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - The default display settings on startup are 4 cycles and zero-cross. Display settings are not saved.

# Build & Operation Check Procedure
- *Prerequisite:* First, please build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for instructions. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI how to install it, and it's easy! The general idea is that it's easy if you let AI handle everything!
- *Next Prerequisite:* If Rust is not installed, ask AI how to install it, and it's easy!
- As mentioned, confirm that `../uapmd` is built.
- Confirm that an audio plugin (instrument) like Surge XT is installed. For example, confirm Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the screen opens.
- Confirm that Surge XT and similar plugins are listed on the screen.
- Click the `Load` button.
- Confirm it loads and plays sound.

# Miscellaneous

The text was generated by AI and is hard to read. I'll fix it later.

## Window Position & Size Settings

In `%LOCALAPPDATA%\cat-plugin-player\config.toml`, you can specify the main window's startup position/size and the plugin GUI's placement method. Please edit it after closing the app. Session and favorite selection states are saved in `status.json` in the same directory, and normal saves do not overwrite the TOML. If the old TOML contains mixed states, they are migrated on first startup, the original file is saved as `config.toml.before-status-migration.bak`, and only the state items are removed. Configuration comments, formatting, and unknown items are preserved, and isolated comments associated with deleted items are left at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates and are used as the startup position if both are specified. If omitted, it uses the traditional initial position. `width` and `height` are the inner width and height of the main window (logical coordinates). Either can be specified; if omitted, or if values are 0 or less, or non-finite, they revert to 900 width and 600 height. Changes are reflected on the next startup. The plugin placement method is currently `right_then_bottom_right` only, and the default behavior is the same if omitted. GUI size and placement are calculated based on the actual window frame, screen work area, and DPI. Automatic saving of manually moved main window or GUI positions, or per-plugin position specification, is not performed.

## Check & Update Latest Version from Command Line

```powershell
cat-plugin-player check
cat-plugin-player update
```

## Note *This text was generated by AI and is hard to read. I plan to revise it later.*
- Without arguments, the GUI starts as usual. `--help` and `--version` are also available.
- `check` compares the exe's build commit with `main` branch of `cat2151/cat-plugin-player` on GitHub. `up-to-date` means they match, `update available` means they don't (it doesn't determine if commits are newer/older or compare uncommitted changes). If successful, the exit code is 0; communication failures etc. result in 1.
- `update` uses `cat-self-update-lib`, the same as the referenced project clap-mml-render-tui. On Windows, it runs `cargo install --force --git https://github.com/cat2151/cat-plugin-player` in a separate console. After starting, this command exits. Check the success or failure of the update in the separate console.
- The update destination is Cargo's installation directory (usually `$CARGO_HOME/bin`, or `$HOME/.cargo/bin` if not set). It does not replace the exe in `target/release`. After completion, manually launch `cat-plugin-player` from the installation directory.
- Updating requires Git, Rust/Cargo, `python`, CMake, Microsoft C++ Build Tools, and UAPMD source. The UAPMD location is selected in the order: environment variable `UAPMD_DIR` → `[build]`'s `uapmd_dir` in `%LOCALAPPDATA%\cat-plugin-player\config.toml` → the location during this exe's build. It is converted to an absolute path and passed to Cargo for the update as environment variable `UAPMD_DIR`. UAPMD itself is not updated.
- You can specify in config as follows (replace the path with your actual checkout location). For relative paths, it's relative to the directory containing config.toml. GUI session saves do not overwrite config.toml.

```toml
[build]
uapmd_dir = 'X:\projects\uapmd'
```

## Note *This text was generated by AI and is hard to read. I plan to revise it later.*
- When running `cargo install --force --git https://github.com/cat2151/cat-plugin-player` directly, it does not read the config. In PowerShell, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. If not set, the build will look for `uapmd` next to the package obtained by Cargo, so placing UAPMD in the current directory where Cargo is run will not resolve it.
- To allow launching from the installation directory, the built shim DLL is bundled with the exe. It's extracted to the user's cache directory and loaded when the GUI starts. `check` / `update` do not launch the GUI, audio, or plugins.

# Future Brainstorming
- *May change based on mood.*
- Leverage the strength of VST3 and GUI support, which `cmrt` lacks.
  - Automated GUI operation?
    - Log GUI operations and reproduce snappy TUI operations with `cmrt`? Provide hints for per-plugin custom handling?
  - Experiment with `egui`.
    - Keyboard display?
    - Grid sequencer?
- Utilize various `cmrt` crates to do various things.
  - *This refines the crates and benefits from their use, making it a win-win.*
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The `cmrt` keyboard screen can play with one command by specifying patch, effect, arpeggio, etc., from the CLI. Make the same possible (to the extent that it plays just by changing `cmrt` to `cat-plugin-player`).
      - Frequent breaking changes are acceptable. Manual migration work is possible later. Better than being stuck and hindering progress for fear of change.

# Concept, What We Aim For
- Instrument
  - For example, a synthesizer with a MIDI keyboard makes sound when you turn it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations to produce sound.
- Sounding
  - We aim to maintain "it sounds on the author's environment" as much as possible.
- Immediate Sound
  - From app launch to sound output, a perceived 0.2 seconds. We prioritize the fastest playback.
    - Processing like GUI display preparation is done after the sound.
      - Sound playing before GUI display could be called a sound version of a splash screen.
    - Requires directly running a release-built exe (via `cargo` adds timestamp checks, and debug builds add debug code, both adding to startup latency).
- Educational Use
  - More oriented towards educational use than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes.

# Random Thoughts Corner
- *Related to the concept.*
- I like audio plugin presets.
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It's the combination of each layer from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and the performance UI.
        - This is a layer where vibecoding is possible.
        - Building this can improve UX.
- For whom is this instrument?
  - Cats, babies.
    - Meaning, the target audience broadly includes beginners, even to the extent of cats and babies (e.g., Android tablets).
  - *This is purely metaphorical. Whether it actually runs on Android tablets is unconfirmed. The likelihood of effort to make it run on Android if it doesn't is also low.*

# Out of Scope, What We Don't Aim For
- Robustness
  - Absolutely no bugs, the app never crashes.
  - Full compatibility. Loads all past settings and data and operates perfectly.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed with no CPU load in all environments. Audio never drops out.
- Requests
  - Immediate response to all user requests.

# License
- MIT