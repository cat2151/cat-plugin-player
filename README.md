# cat-plugin-player

A lightweight application for easily playing audio plugins, built with [UAPMD](https://github.com/atsushieno/uapmd) and written in Rust.

# Previous Challenges and What This App Solves
- Audio plugin sounds are fantastic!
- To play them, you often need to launch a DAW and perform various setup steps.
- Existing lightweight apps for easy playback sometimes require account registration or have a user experience that just isn't "for me".
- Today, if you have a desired UX, you can achieve it through "vibecoding".
- So, that's what we decided to do.

# Features
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button automatically starts playback after the plugin is loaded.
- GUI
  - You can display the plugin's GUI by pressing the `Show UI` button.
- Sound
  - To change the sound, use the plugin's GUI.

# Features (Detailed additions by AI, to be refined later as it's a bit verbose)
- Playback
  - The plugin instance last played is saved to `%LOCALAPPDATA%\cat-plugin-player\config.toml` and automatically regenerated and played back on the next launch.
  - It is remembered upon successful `Load`, when `Sequence` is turned on, or when playing a `Test note`. This memory persists even after stopping or `Remove`ing the plugin.
  - What's saved is the plugin's format and ID. It does not include saving changes to sounds or parameters made within the plugin itself.

# Build & Verification Steps
- **Prerequisite:** First, build [UAPMD](https://github.com/atsushieno/uapmd). Please refer to its documentation for instructions. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI to install it – it's easy! The overall idea is to let AI handle it.
- **Next Prerequisite:** If you don't have Rust, ask AI to install it – it's easy!
- As mentioned above, confirm that `../uapmd` has been built.
- Confirm that an audio plugin (instrument) like Surge XT is installed. For example, verify that Surge XT produces sound in REAPER.
- Run `cargo run` (the application will be debug-built).
- Confirm that the window opens.
- Confirm that plugins like Surge XT are listed on the screen.
- Click the `Load` button.
- Confirm that the plugin loads and sound is produced.

# Future Brainstorming
- *Subject to change based on mood.*
- Plugin support
  - Many plugins detectable by `cmrt` are not detected by this app.
    - CLAP plugins are an example; only about 2 out of 10 are detected.
- Effect routing
  - Start small. First one effect, then serial. `cmrt` has already implemented this.
- Extend strengths not present in `cmrt`, such as VST3 and GUI support.
  - Screenshots
    - Save screenshots of opened GUIs to the config directory, and the plugin launcher displays them for visual selection.
      - Previous problem: Choosing plugins based on text makes it hard to quickly distinguish them.
      - Images: Might allow for intuitive selection, reducing visual fatigue. Worth investigating.
    - Consideration: `cmrt`'s method of filtering by patch name across plugins is also good. Plan to verify coexistence.
  - Automatic GUI operation?
    - Log GUI operations and reproduce them quickly in `cmrt`'s TUI? As a hint for plugin-specific adaptations?
  - Play around with `egui`
    - Keyboard display?
    - Waveform display?
    - Grid sequencer?
  - Snapshot, state save
    - Recall a favorite sound with a specific type of shimmer reverb inserted, all with a single hotkey.
- Leverage various `cmrt` crates to do more things.
  - *This will also refine the crates, and we benefit from using them, so it's a win-win.*
  - Patch selector
    - Can detect preset patches of CLAP audio plugins.
  - CLI import / export
    - The `cmrt` keyboard screen can play a patch, effect, arpeggio, etc., from the CLI with a single command. Aim to achieve the same (to the level where just changing `cmrt` to `cat-plugin-player` makes it work).
      - Frequent breaking changes are acceptable. Manual migration is possible later. It's better than halting progress out of fear of change.

# Concepts, Goals
- Instrument
  - For example, a synthesizer with a MIDI keyboard makes sound when you turn it on and press keys.
    - We aim for that level of ease of use.
      - Minimize operations to produce sound.
- Playability
  - Aim to maintain "it works on the author's machine" as much as possible.
- Educational
  - Leans more towards educational use rather than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

# Casual Thoughts Corner
- *Related to concepts*
- I love audio plugin preset sounds
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide an instrument UX
  - I want an instrument UX, not a DAW UX.
- What is an instrument?
  - It is the sum of layers from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and the performance UI.
        - This is a layer where "vibecoding" can be done.
        - Building this can improve UX.
- For whom is this instrument?
  - Cats, babies.
    - In other words, we aim to expand the target range to beginners, even including cats and babies (e.g., on Android tablets).
  - *This is purely a metaphor. It is unconfirmed whether it works on Android tablets. The probability of effort to make it work on Android if it doesn't is also low.*

# Out of Scope, Not Aimed For
- Robustness
  - Never bugs, never crashes, no matter what you do.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Convenience
  - Pursuing ultimate convenience and ease of use.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never drops out.
- Requests
  - Immediate response to all user requests.

# License
- MIT