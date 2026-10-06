# cat-plugin-player

A lightweight application that allows you to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Problems So Far and What This App Solves
- Audio plugin sounds are fantastic!
- To play them, you often need to launch a DAW and perform setup beforehand.
- Lightweight apps for easy playback sometimes required account registration or had a user experience that wasn't "for me."
- In modern times, if you have a desired UX, you can achieve it through "vibe coding."
- So, I decided to do just that.

# Features
- **Detection**
  - Upon launch, it automatically detects installed plugins.
- **Playback**
  - Pressing the `Load` button automatically starts playback after the plugin is loaded.
- **GUI**
  - You can display the plugin's GUI by pressing the `Show UI` button.
- **Timbre**
  - To change the timbre, use the plugin's GUI.

# Features (Detailed additions by AI. Intending to refactor later as it's a bit hard to read)
- **Playback**
  - The plugin instance last played is saved to `%LOCALAPPDATA%\cat-plugin-player\config.toml`, regenerated upon next launch, and automatically played.
  - It remembers when `Load` is successful, `Sequence` is turned on, or `Test note` is played. The memory persists even after stopping or `Remove`ing.
  - Saved targets include plugin format, ID, name, vendor, and file location. It does not include saving timbre or parameters modified within the plugin.
  - Upon startup, it directly loads the previous plugin and starts playback, then initializes the GUI and scans the list in the background.
  - If direct loading is not possible due to old history or file relocation, it restores from the scan results after the GUI is displayed. Old history is updated once played.
  - For startup time measurement methods and results, please refer to [docs/startup-timing.md](docs/startup-timing.md).

# Build & Verification Steps
- *Prerequisite*: First, please build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for instructions. On Windows, it's easy if you have `Microsoft C++ Build Tools` installed! If not, ask an AI how to install it – it's that easy! The general idea is to let AI handle it.
- *Next Prerequisite*: If Rust is not installed, ask an AI how to install it – it's that easy!
- As mentioned, ensure that `../uapmd` is built.
- Ensure that an audio plugin (instrument) like Surge XT is installed. For example, verify that Surge XT produces sound in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the window opens.
- Confirm that Surge XT and similar plugins are listed on the screen.
- Click the `Load` button.
- Confirm that the plugin loads and produces sound.

# Future Brainstorming
- *Subject to change based on mood*
- **Plugin Compatibility**
  - Many plugins detectable by `cmrt` are not detected by this app.
    - For instance, out of about 10 CLAP plugins, only 2 are detected.
- **Effect Routing**
  - Start small. First one, then serial. `cmrt` has already implemented this.
- **Leverage strengths not found in `cmrt` (VST3 and GUI support)**
  - **Screenshots**
    - Save screenshots of open GUIs to the config directory, and the plugin launcher displays them for visual selection.
      - Previous issue: Selecting plugins from a text-based list is prone to "eye-slipping" (losing focus).
        - With images, selection might be intuitive, and "eye-slipping" might be reduced. Worth investigating.
    - Consideration: `cmrt`'s method of selecting across plugins by filtering patch names is also good in its own way. We envision coexistence and validation.
  - **GUI automation?**
    - Log GUI operations and reproduce them swiftly via `cmrt`'s TUI? Could this hint at plugin-specific adaptations?
  - **Experiment with `egui`**
    - Keyboard display?
    - Waveform display?
    - Grid sequencer?
  - **Snapshot, State Save**
    - Call up a favorite sound with a preferred shimmer reverb inserted, all with a single shortcut key.
- **Utilize various `cmrt` crates for diverse functionalities**
  - *This would also refine the crates, and we benefit from using them, making it a win-win.*
  - **Patch Selector**
    - Can detect preset patches for CLAP audio plugins.
  - **CLI Import / Export**
    - The `cmrt` keyboard screen can play a sound instantly from the CLI by specifying patch, effect, arpeggio, etc. Enable the same functionality (to the level where changing `cmrt` to `cat-plugin-player` in the command makes it work).
      - Frequent breaking changes are acceptable. Manual migration is possible later. Better than fearing changes, becoming immobile, and hindering progress.

# Concept, What We Aim For
- **Musical Instrument**
  - For example, a synthesizer with a MIDI keyboard produces sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize the operations required to produce sound.
- **Sound Production**
  - We aim to maintain "it produces sound in the author's environment" as much as possible.
- **Instant Sound**
  - From app launch to sound production, the perceived time is 0.2 seconds. We prioritize the fastest possible playback.
    - Processes like GUI display preparation are done after the sound is produced.
      - Sound playing before the GUI appears might be considered a "sound splash screen."
- **Educational Use**
  - It is geared more towards educational purposes than commercial use.
- **Experimentation, Exploration, Personal Use**
  - We will frequently make breaking changes.

# Random Thoughts Corner
- *Related to the concept*
- I love audio plugin preset sounds.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide an instrument UX.
  - I want to provide a musical instrument UX, not a DAW UX.
- What is a musical instrument?
  - It's a synthesis of layers from the audio plugin to the PC keyboard.
    - There is an intermediate layer.
      - This is the plugin host and performance UI.
        - This is a layer that can be "vibe coded."
        - Creating this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - In other words, we aim to broaden the target audience to include beginners, even to the extent of cats and babies (e.g., on Android tablets).
  - *This is merely a metaphor. It is unconfirmed whether it actually works on Android tablets. The likelihood of effort being made to make it work on Android if it doesn't is also low.*

# Out of Scope, What We Don't Aim For
- **Robustness**
  - Absolutely no bugs, no crashes, no matter what.
  - Full compatibility. Loads and operates with all past settings and data.
- **Features**
  - Equipped with every conceivable feature. Usable for all purposes.
- **Convenience**
  - Pursuing convenience and ease of use to the extreme.
- **Performance**
  - Operates at high speed without CPU load in all environments. Sound never drops out.
- **Demands**
  - Instant response to all user requests.

# License
- MIT