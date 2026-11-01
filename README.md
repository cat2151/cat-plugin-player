# cat-plugin-player

A lightweight application that allows you to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

## Previous Challenges and What This App Solves
- Audio plugin presets sound amazing!
- To play them, it often requires launching a DAW and performing preliminary setup.
- Lightweight apps for easy playback sometimes required account registration to obtain, or their user experience/feel wasn't for me.
- Nowadays, if you have a desired UX, you can achieve it through 'vibecoding'.
- That's what I decided to do.

## Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

## Demo Video

If you haven't installed any audio plugins, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) for you, as shown here. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

## Features
- Automatic Installation
  - If you haven't installed any audio plugins, the app automatically installs [Surge XT](https://surge-synthesizer.github.io/) as mentioned above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking on a plugin automatically starts playback after loading the plugin.
  - Chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde` can also be played.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit presets and effects.
- Presets
  - Each plugin has a variety of preset sounds. You can easily select them using a unique preset browser (supported for some plugins only).
  - For plugins not supported by the preset browser, you can simply select presets via the plugin's GUI.
  - By default, only Surge XT is supported.
    - It can also support Dexed, etc. When you create a preset catalog by running https://github.com/cat2151/clap-mml-render-tui, this app automatically utilizes it. It supports a total of over 30,000 presets across various plugins.
- Effects
  - Each plugin has a variety of preset effects. You can easily select them using a unique effect browser (supported for some plugins only).
  - For plugins not supported by the effect browser, you can simply select effects via the plugin's GUI.

## Future Brainstorming
- ※ Subject to change based on mood/priorities.
- Extend strengths not present in [cmrt](https://cat2151.github.io/clap-mml-render-tui/README.ja), such as VST3 and GUI support.
  - Integrate GUI edit results/status with cmrt.
    - Load and use edited presets and effects in cmrt.
  - Experiment with egui for various features.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, ensemble playback of multiple SMFs, swapping presets and effects?
    - Should these be extracted into separate apps? Or is it better to do them within this app given the preset/effect swapping? What kind of workflow is envisioned?
- Utilize various cmrt crates for different functionalities.
  - ※ This will also refine the crates themselves, and since we benefit from using them, it's a win-win.
  - CLI import / export
    - The cmrt keyboard screen can instantly play patches, effects, arpeggios, etc., specified via CLI. Enable the same functionality (to the level where changing 'cmrt' to 'cat-plugin-player' is enough to get sound).
    - Frequent breaking changes are acceptable. Manual migration is possible later. Better than fearing changes, becoming immobile, and halting progress.
  - SMF import / export
    - Currently, outputting SMF with audio plugin and patch names recorded is already implemented, and we aim to expand upon it.

## Concept & Goals
- Musical Instrument
  - For example, a synthesizer with a MIDI keyboard produces sound by simply turning it on and pressing a key.
    - We aim for that level of ease of use.
      - Minimize operations required to produce sound.
- Sound Production
  - We aim to maintain 'sound production in the author's environment' as much as possible.
- Instant Playback
  - Perceived 0.2 seconds from app launch to sound. Prioritizing the fastest path to playback.
    - Processes like GUI display preparation are also deferred until after sound production.
      - Sound playing before the GUI appears could be considered the sound equivalent of a splash screen.
- Educational Use
  - Oriented more towards educational use than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

## Random Thoughts Corner
- ※ Related to the concept.
- I like audio plugin preset sounds.
  - Because they are reproducible.
  - Thanks to being shared freely, everyone can benefit.
- I want to provide a musical instrument UX.
  - Provide a musical instrument UX, not a DAW UX.
- What is a musical instrument?
  - It is the sum of each layer from audio plugin to PC keyboard.
    - There is an intermediate layer.
      - That is the plugin host, the performance UI.
        - This is the layer where 'vibecoding' can be applied.
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - In other words, broaden the target audience towards beginners, even to the extent of including cats and babies (e.g., on Android tablets).
  - ※ This is purely a metaphorical expression. It is unconfirmed whether it actually works on Android tablets. The likelihood of effort being made to make it work on Android if it doesn't is also low.

## Out of Scope & What We Don't Aim For
- Robustness
  - Absolutely no bugs, no matter what you do. The app never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Convenience
  - Pursuing ultimate convenience and ease of use.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never drops out.
- Requests
  - Immediately responding to all user requests.

## Miscellaneous
### To specify the main screen coordinates and size at startup

In `settings.toml` in AppData Local, for example, write the following:

```
window = [0, 0, 1800, 900]
```

## License
- MIT