# cat-plugin-player

A lightweight app for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Previous Challenges and What This App Solves
- Audio plugin timbres are amazing!
- To play them, it often requires launching a DAW and performing preliminary setup for playback.
- Lightweight apps for easy playback sometimes require account registration to obtain, or their operation felt "not for me."
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So, I decided to do just that.

# Features
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button automatically starts playback after the plugin is loaded.
- GUI
  - You can display the plugin's GUI by pressing the `Show UI` button.
- Sound
  - To change the sound/timbre, use the plugin's GUI.

# Features (Detailed additions by AI. Will revise later as it's a bit hard to read)
- Playback
  - The last played plugin instance is saved to `%LOCALAPPDATA%\cat-plugin-player\config.toml` and regenerated for automatic playback on the next launch.
  - It remembers when `Load` is successful, when `Sequence` is turned on, or when a `Test note` is played. The memory persists even after stopping or `Remove`.
  - Saved items include the plugin's format, ID, name, vendor, and file location. This does not include saving timbre or parameters changed within the plugin.
  - On startup, it directly loads the previous plugin and starts playback, then initializes the GUI and scans the list in the background.
  - If direct loading is not possible due to old history or file relocation, it is restored from the scan results after GUI display. Old history is updated once played.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.

# Build & Test Procedures
- *Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Please refer to its documentation for procedures. On Windows, it's easy if you have `Microsoft C++ Build Tools` installed! If not, it's easy to install by asking AI! The general idea is that AI makes everything easy!*
- *Next prerequisite: If Rust is not installed, it's easy to install by asking AI!*
- As mentioned, confirm that `../uapmd` is already built.
- Confirm that an audio plugin (instrument) like Surge XT is installed. For example, confirm that Surge XT plays sound in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the window opens.
- Confirm that Surge XT, etc., are listed on the screen.
- Click the Load button.
- Confirm that it loads and produces sound.

# Future Brainstorming
- *May change depending on my mood.*
- Plugin compatibility
  - Many plugins that `cmrt` can detect are not detected by this app.
    - CLAP plugins are an example; only about 2 out of 10 are detected.
- Effect routing
  - Start small. First one. Then serial. `cmrt` already achieves this.
- Extend the unique strength of supporting VST3 and GUI, which `cmrt` lacks.
  - Screenshots
    - Save screenshots of opened GUIs to the config directory, and the plugin launcher displays them, allowing visual selection.
      - Previous challenge: choosing plugins based on text makes the eyes glaze over.
        - With images, selection can be intuitive. It might prevent the eyes from glazing over. Worth testing.
    - Consider: `cmrt`'s method of selecting plugins across the board by filtering patch names is also good in its own way. The plan is to verify both approaches concurrently.
  - Automatic GUI operation?
    - Log GUI operations and reproduce snappy operations in `cmrt`'s TUI? As a hint for plugin-specific adaptations?
  - Play around with `egui`
    - Keyboard display?
    - Waveform display?
    - Grid sequencer?
  - Snapshot, state save
    - Recall a favorite timbre with a preferred type of shimmer reverb inserted, with a single shortcut key.
- Utilize various `cmrt` crates for various functionalities.
  - *This also refines the crates, and we benefit from using them, so it's a win-win.*
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The `cmrt` keyboard screen can play a sound with one command from the CLI, specifying patch, effect, arpeggio, etc. Make the same possible (to the level where changing `cmrt` to `cat-plugin-player` is all it takes to play).
      - Frequent destructive changes are acceptable. Manual migration is possible later. It's better than fearing changes, becoming stuck, and halting progress.

# Concept, What It Aims For
- Instrument
  - For example, a synthesizer with a MIDI keyboard plays a sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations required to produce sound.
- Sound Production
  - We aim to maintain, as much as possible, "that it plays in the author's environment."
- Instant Playback
  - Perceived 0.2 seconds from app launch to sound. Playback is optimized for maximum speed.
    - Processing like GUI display preparation is also done after the sound.
      - Sound playing before the GUI appears might be considered a sound version of a splash screen.
    - You need to run the release-built .exe directly (running via cargo involves timestamp checks, and debug builds include debug code, both of which add to startup time).
- Educational Use
  - It is oriented more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - Frequent destructive changes will be made.

# Casual Thoughts Corner
- *Related to the concept*
- I like the preset sounds of audio plugins.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It is the sum of each layer from audio plugins to PC keyboards.
    - There is an intermediate layer.
      - That is the plugin host, the playback UI.
        - This is a layer that can be "vibe-coded."
        - By creating this, UX can be improved.
- For whom is this instrument?
  - Cats, babies
    - In other words, the target range is broadened towards beginners, even including cats and and babies (e.g., on Android tablets).
  - *This is merely a metaphor. Whether it actually runs on Android tablets is unconfirmed. The likelihood of effort being made to make it work on Android if it doesn't is also low.*

# Out of Scope, Not Aimed For
- Robustness
  - Absolutely no bugs, no app crashes, no matter what.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with all conceivable features. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation with no CPU load in any environment. Sound will never drop out.
- User Requests
  - Immediately responding to all user requests.

# License
- MIT