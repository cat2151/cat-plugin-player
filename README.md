# cat-plugin-player

A lightweight application to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- Audio plugins have amazing sounds!
- To play them, it often requires launching a DAW and performing preliminary setup for playback.
- Lightweight apps for easy playback sometimes require account registration to obtain, or their operation felt "not for me."
- Nowadays, if you have a desired UX, you can achieve it through vibe coding.
- So, I decided to do just that.

# Features

The oscilloscope display method and constraints are documented in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking the `Load` button automatically starts playback after the plugin is loaded.
- GUI
  - Clicking the `Show UI` button displays the plugin's GUI.
  - The first time the GUI is opened, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a maximum of 192x128 PNG, preserving the aspect ratio. It saves to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`. Plugins with matching names + manufacturers share the same image across VST3 / CLAP (ignoring leading/trailing spaces and case). If name/manufacturer are missing, they are distinguished by format + ID.
  - Previously saved format-specific images are automatically migrated from the list/favorites identification information to shared images and used. The tone state and favorite load destination are still distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Hovering over an image shows it at its saved size. Plugins with no displayed GUI show a placeholder icon.
  - For blank images or failed captures, it retries at 1-second intervals for up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured, and opening the GUI replaces them with valid images. No re-capturing occurs after success.
  - If capturing fails after retries, an error is displayed. Reopening the GUI retries. GUIs that do not support capturing remain with placeholder icons. If you want to retake an image or if the saved PNG is corrupted, delete the relevant plugin's shared PNG and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are left during migration).
- Effect Routing
  - Serial connection: one instrument → one effect → output. Simply load an instrument, then press `Load` on an effect to connect it.
  - `Routing` displays the connection order. You can edit the sound or effect via `Show UI` for each plugin.
  - `Remove` on an effect reverts to a direct connection from the sound source to the output, and `Bypass effect` allows audio to pass through while retaining the instance and settings.
  - `Sequence` is for instruments. Audio processing continues even if the Sequence is stopped, allowing for effect tails.
  - On the next launch, the instrument, effect, their respective states, and Bypass status are restored, then playback begins. If effect restoration fails, an error is displayed, it reverts to a direct sound source connection, and the saved configuration is retained.
  - Supports mono/stereo for main input/output. Mono → stereo is duplicated, stereo → mono is the average of left and right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause a brief audio glitch, and notes being played and tails may be cut off. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Tones/Presets
  - To change a tone, use the plugin's GUI.
  - Using `★ Add favorite` for each plugin, you can save multiple favorites of the current tone, settings, and playback pattern (including stopped state). Names are assigned automatically and can be changed later.
  - When you `Load` a different plugin, the tone, settings, and playback pattern of the plugin being replaced are automatically saved to Favorites before the switch. The name will be `auto PluginName SequenceNumber`. It will not add entries for reloading the same plugin or restoring on startup. If saving fails, the switch is aborted.
  - During manual and automatic saving, it checks for duplicates of the same plugin, role, playback pattern, and state, keeping the new favorite and deleting old entries and save files.
  - Refer to [Plugin-Specific Processing Policy and List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) for product-specific exceptions, reasons, scope, and limitations. For the confirmed state format of TyrellN6 CLAP, the compressed part that changes only with playback is excluded from duplicate determination, and tones with the same text settings are grouped. For the confirmed format of Vaporizer2 CLAP, minute differences in MSEG time and coordinates (absolute difference of specified items `<=1e-9`) caused by loading/saving are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - The `Favorites` / `Plugins` tabs in the center switch the list view. Favorites are displayed from newest to oldest and can be filtered by favorite name, plugin name, and format.
  - Favorites are displayed one per line. Long names and supplementary information are truncated, and hovering over them shows the full text. Waveforms and spectrum are displayed under Routing on the right; the width of the right panel can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles the analysis display between bottom-right / bottom of the screen (defaulting to bottom-right on startup).
  - A colored `Instrument` / `Effect` is displayed to the left of each row. A single click on a favorite name restores the tone, settings, and playback pattern. Favorites saved in a stopped state are restored as stopped. Instrument favorites maintain the connected effect, and effect favorites maintain the instrument. An instrument is required to recall an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed, and clicking the name again removes the effect. This behavior is the same after editing settings or during Bypass. The instrument and saved favorite remain. Clicking a different effect favorite switches to its settings.
  - `...` at the end of the row allows renaming and deleting. Edits after recalling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved or recalled favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. There may be a short audio glitch during saving and restoring, and the saveable range is the same as traditional state saving.

# Features (Detailed addition by AI. Intending to refine later as it's hard to read)
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen shows the output's L (green) and R (blue) channels superimposed, reflecting effects and Bypass. Mono displays the same waveform on both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), allowing selection of `1`, `2`, `4`, `8 cycles`. After Note Off, the tail is displayed based on the last note. Output containing chords or detuning may not perfectly repeat at that width.
  - `Zero cross` aligns with the negative to positive crossing of the left channel. If the left channel is silent, the right channel is used; if no crossing, it displays from the beginning.
  - `Similarity` exhaustively scans from the beginning of the buffer by 1 sample increment for the display width, finding the position with the maximum correlation coefficient to the previous display. Ties are resolved by choosing the earlier position, and the same starting position is used for left and right. For the first display or when the comparison target is silent, zero crossing is used.
  - The comparison history is reset when a note is played, the playback pattern changes, the number of display cycles changes, the trigger method changes, or the audio stream changes. It will show a standby display until sufficient data is collected.
  - Exploration runs on a separate thread from audio processing. Display updates may be slow for low notes or many cycles. If display data is interrupted, the history is reset.
  - The initial display settings on startup are 4 cycles and zero-crossing. Display settings are not saved.
- Playback
  - The last played instrument, connected effect, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\config.toml` and restored on the next startup. The stopped state is also restored.
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. Arrows cycle through the types, and "stop" is not included in the types. Switching during playback releases all notes in the current phrase and starts from the beginning of the new phrase. While stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop. The selected type and stopped state are restored on the next startup.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, or `Random` (random) using the same left/right arrows or dropdown. The initial value is 100. Random selects 1-127 for each Note On. Changes take effect from the next note played without restarting the phrase and are restored on the next startup.
  - Guitar open strings (MIDI note numbers 40, 45, 50, 55, 59, 64 for E, A, D, G, B, E) are Note On sequentially at 125ms intervals. All six notes are Note Off simultaneously 2 seconds after the last Note On, and the sequence repeats after 0.5 seconds.
  - This is remembered on successful `Load`, and when selecting, starting, or stopping a `Sequence` type. The memory persists even after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, the binary state of sounds, parameters, etc., disclosed by the plugin via UAPMD's state API is saved per plugin to `%LOCALAPPDATA%\cat-plugin-player\states\`. On the next startup or when reloading the same plugin, it is restored before playback begins.
  - `Load` for an instrument replaces only the sound source, maintaining the connected effect and settings. The state of the plugin being replaced is saved, and it switches only after the new plugin is successfully generated, restored, and connected. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. It also requests UI state, but the current UAPMD VST3 implementation only retrieves component state. It does not include replication of external samples or saving during forced termination.
  - On startup, it directly loads the previous instrument and effect, restores their states, and then starts playback. Afterwards, it initializes the GUI and scans the list in the background.
  - If direct loading fails due to old history or file moves, it restores from the scan results after the GUI is displayed. Old history is updated once played.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.

# Build & Test Procedures
- ※ As a prerequisite, first build [UAPMD](https://github.com/atsushieno/uapmd). Please refer to its documentation for instructions. On Windows, it's easy if you have `Microsoft C++ Build Tools` installed! If not, just ask AI how to install it, and it's easy! The overall image is "easy if you let AI handle it all."
- ※ As a next prerequisite, if Rust is not installed, it's easy if you ask AI how to install it! That's the image.
- As mentioned, ensure that `../uapmd` is already built.
- Ensure that an audio plugin (instrument) like Surge XT is installed. For example, confirm Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the window opens.
- Confirm that Surge XT, etc., are listed on the screen.
- Click the `Load` button.
- Confirm that it loads and sound plays.

# Check & Update Latest Version from Command Line

```powershell
cat-plugin-player check
cat-plugin-player update
```

## Supplemental Notes ※ Text generated by AI, so it might be hard to read. Intending to refine later.
- Without arguments, it launches the GUI as usual. `--help` and `--version` are also available.
- `check` compares the exe's build-time commit with `main` of `cat2151/cat-plugin-player` on GitHub. `up-to-date` means they match, `update available` means they don't (it doesn't determine if a commit is newer/older or compare uncommitted changes). If successful, the exit code is 0; communication failures, etc., result in 1.
- `update` uses `cat-self-update-lib`, the same as the referenced project clap-mml-render-tui. On Windows, it runs `cargo install --force --git https://github.com/cat2151/cat-plugin-player` in a separate console. After starting, this command exits. Check the success or failure of the update in the separate console.
- The installation target is Cargo's install location (usually `$CARGO_HOME/bin`, or `$HOME/.cargo/bin` if not set). It does not replace the exe in `target/release`. After completion, manually launch `cat-plugin-player` from the install location.
- Updating requires Git, Rust/Cargo, `python`, CMake, Microsoft C++ Build Tools, and UAPMD source. The location of UAPMD is chosen in order: `UAPMD_DIR` environment variable → `[build]` `uapmd_dir` in `%LOCALAPPDATA%\cat-plugin-player\config.toml` → the location at the time of this exe's build. It is converted to an absolute path and passed to Cargo for update as the `UAPMD_DIR` environment variable. UAPMD itself is not updated.
- You can specify in config as follows (replace the path with your actual checkout): For relative paths, it's relative to the directory containing config.toml. Build settings are retained even when saving GUI sessions.

```toml
[build]
uapmd_dir = 'X:\projects\uapmd'
```

## Supplemental Notes ※ Text generated by AI, so it might be hard to read. Intending to refine later.
- When directly running `cargo install --force --git https://github.com/cat2151/cat-plugin-player`, it does not read the config. If using PowerShell, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. If unset, the build will look for `uapmd` next to the package fetched by Cargo, so placing UAPMD in the current directory where Cargo is run will not resolve it.
- To enable launching from the install location, the built shim DLL is bundled with the exe. When the GUI starts, it is extracted to the user's cache directory and loaded. `check` / `update` do not launch the GUI, audio, or plugins.

# Future Brainstorming
- ※ May change depending on mood.
- Effect Routing
  - Serial connection of multiple effects. Already implemented in cmrt.
- Enhance the strength of VST3 and GUI support, which cmrt lacks.
  - Screenshots
    - Save screenshots of opened GUIs to the config directory, and the plugin launcher displays them, allowing visual selection.
      - Previous challenge: selecting plugins text-based is hard on the eyes.
        - With images, selection can be intuitive. Less eye strain, perhaps. Worth validating.
    - Consideration: the cmrt method of filtering by patch name across plugins is also good. Intended to coexist and be validated.
  - Automatic GUI operation?
    - Log GUI operations and reproduce snappy operations in TUI with cmrt? As a hint for plugin-specific handling?
  - Play around with egui.
    - Keyboard display?
    - Grid sequencer?
  - Snapshot, state save
    - Recall a favorite tone with a favorite type of shimmer reverb inserted with a single shortcut key.
- Utilize various cmrt crates to do various things.
  - ※ This also refines the crates, and we benefit from using them, so it's a win-win.
  - Patch selector
    - Can detect preset patches of CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can play a patch, effect, arpeggio, etc., with a single command from the CLI. Make the same possible (to the level where it plays by just changing cmrt to cat-plugin-player).
      - Frequent destructive changes are acceptable. Manual migration is possible later. Better than being paralyzed by fear of change and stopping progress.

# Concepts, What It Aims For
- Instrument
  - For example, a synthesizer with a MIDI keyboard produces sound when you power it on and press a key.
    - It aims to be as easy to use as that.
      - Minimize operations to produce sound.
- Producing Sound
  - Aims to maintain "it works on the author's environment" as much as possible.
- Immediate Sound
  - From app launch to sound production, it's a perceived 0.2 seconds. Playback is prioritized for speed.
    - Processes like GUI display preparation are done after the sound.
      - Sound playing before the GUI appears might be considered the audio version of a splash screen.
    - Requires directly executing a release-built exe (as `cargo` adds timestamp checks, and debug builds add debug code, both causing launch delays).
- Educational
  - It leans more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - Frequent destructive changes will occur.

# Miscellaneous Thoughts Section
- ※ Related to the concept.
- I love audio plugin preset sounds.
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It's the aggregate of each layer from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host, the playback UI.
        - This is a layer that can be "vibe coded."
        - Creating this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - Meaning, it targets beginners to the extent that it includes cats and babies (e.g., an Android tablet).
  - ※ This is merely a metaphorical expression. Whether it actually runs on an Android tablet is unconfirmed. The likelihood of effort being made to make it run if it doesn't is also low.

# Out of Scope, What It Does Not Aim For
- Robustness
  - Absolutely no bugs, the app never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipping every possible feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed without CPU load in all environments. Audio never cuts out.
- Demands
  - Immediately responding to all user requests.

# License
- MIT