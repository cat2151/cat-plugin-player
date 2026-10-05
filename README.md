# cat-plugin-player

A lightweight app for easily playing audio plugins. It uses [UAPMD](https://github.com/atsushieno/uapmd). It is written in Rust.

# Past Challenges and What This App Solves
- Audio plugins have amazing timbres!
- To play them, you often need to launch a DAW and perform setup steps.
- Lightweight apps for casual playback sometimes require account registration for acquisition, or their user experience wasn't to my liking.
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding".
- So I decided to do just that.

# Features
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button automatically starts playback after the plugin is loaded.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI.
- Timbre
  - To change the timbre, use the plugin's GUI.

# Build & Verification Steps
- ※ Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Please refer to its documentation for build instructions. On Windows, it's easy if you have `Microsoft C++ Build Tools` installed! If not, just ask an AI how to install it – it's that easy! The general idea is to let AI handle everything.
- ※ Next prerequisite: If Rust isn't installed, just ask an AI how to install it – it's that easy!
- As mentioned, confirm that `../uapmd` has been built.
- Confirm that an audio plugin (instrument) like Surge XT is installed. For example, confirm that Surge XT can be played in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the screen opens.
- Confirm that Surge XT and similar plugins are listed on the screen.
- Click the Load button.
- Confirm that it loads and sound is produced.

# Future Brainstorming
- ※ Subject to change depending on mood/priority.
- Plugin Compatibility
  - There are many plugins that `cmrt` can detect, but this app cannot.
    - CLAP plugins are an example; only about 2 out of 10 are detected.
- Effect Routing
  - Start small. First one, then series. `cmrt` has already implemented this.
- Extend the strengths not present in `cmrt`, such as VST3 and GUI support.
  - Screenshots
    - Save screenshots of open GUIs to the config directory, and the plugin launcher displays them for visual selection.
      - Previous challenge: Choosing plugins from a text-based list is hard on the eyes.
        - With images, you can choose intuitively. It might prevent eye strain. Worth verifying.
    - Consider: The `cmrt` method of filtering and selecting by patch name across plugins is also good. The plan is to verify both approaches in parallel.
  - Automated GUI Operation?
    - Log GUI operations and reproduce them quickly via TUI in `cmrt`? Could this hint at plugin-specific adaptations?
  - Experiment with egui
    - Keyboard display?
    - Waveform display?
    - Grid sequencer?
  - Snapshot, State Save
    - Recall a favorite timbre with a preferred type of shimmer reverb inserted, all with a single shortcut key.
- Utilize various `cmrt` crates for different purposes.
  - ※ This also refines the crates, and we benefit from using them, so it's a win-win.
  - Patch Selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI Import / Export
    - The `cmrt` keyboard screen can play a patch, effect, arpeggio, etc., with a single command from the CLI. Make the same possible here (to the level where changing `cmrt` to `cat-plugin-player` is enough to play).
      - Frequent breaking changes are acceptable. Manual migration is possible later. It's better than fearing changes, becoming stuck, and halting progress.

# Concept, Goals
- Musical Instrument
  - For example, a synthesizer with a MIDI keyboard produces sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize the operations required to produce sound.
- Sound Production
  - We aim to maintain "sound production in the author's environment" as much as possible.
- Educational Use
  - It is oriented more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will be made.

# Random Thoughts Corner
- ※ Related to the concept
- I like the preset timbres of audio plugins.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide a musical instrument UX.
  - Not a DAW UX, but a musical instrument UX.
- What is a musical instrument?
  - It is the sum of each layer from the audio plugin to the PC keyboard.
    - There is an intermediate layer.
      - That is the plugin host, the performance UI.
        - This is a layer where "vibe coding" can be done.
        - By creating this, the UX can be improved.
- For whom is this instrument?
  - Cats, babies.
    - In other words, the target audience's range extends broadly towards beginners, even encompassing cats and babies (e.g., on an Android tablet).
  - ※ This is purely a metaphor. Actual operation on an Android tablet is unconfirmed. The likelihood of effort being made to make it work on Android if it doesn't is also low.

# Out of Scope, Not Aimed For
- Robustness
  - Absolutely no bugs, no app crashes, no matter what.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation with no CPU load in any environment. Sound will never drop out.
- Feature Requests
  - Immediately responding to all user requests.

# License
- MIT