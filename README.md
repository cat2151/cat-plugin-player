# cat-plugin-player

A lightweight app that makes it easy to play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

## Challenges Addressed by This App
- Audio plugins offer incredible sounds!
- To play them, you often need to launch a DAW and perform setup beforehand.
- Lightweight apps for casual playing sometimes require account registration to obtain, or their operation feels "not for me."
- Today, if you have a desired UX, you can achieve it through "vibe coding."
- That's what we decided to do.

## Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

## Demo Video

If you haven't installed any audio plugins, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as shown here. It's easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

## Features
- Automatic Installation
  - If you haven't installed any audio plugins, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as mentioned above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking a plugin automatically starts playback after loading the plugin.
  - It can also play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit sounds and effects.
- Sounds
  - Each plugin has a variety of preset sounds. You can easily select them using a unique sound browser (supported by some plugins only).
  - For plugins not supported by the sound browser, you can select sounds directly via the plugin's GUI.
  - By default, only Surge XT is supported.
    - It can also support Dexed and others. When you create a sound catalog by running https://github.com/cat2151/clap-mml-render-tui, it will be automatically used. In total, it supports over 30,000 sounds across various plugins.
- Effects
  - Each plugin has a variety of preset effects. You can easily select them using a unique effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects directly via the plugin's GUI.

## Future Brainstorming
- *Subject to change based on mood/priorities.
- Enhance the strengths not found in [cmrt](https://cat2151.github.io/clap-mml-render-tui/README.ja), such as VST3 and GUI support.
  - Integrate GUI edit results/status with cmrt.
    - Load and utilize edited sounds and effects in cmrt.
  - Experiment with egui.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, ensemble playing with multiple SMFs, swapping sounds and effects?
    - Should these be separated into different apps? Or is it better to do them in this app because of sound and effect swapping? What workflow is envisioned?
- Utilize various cmrt crates for diverse functionalities.
  - *This refines the crates and provides benefits by using them, making it a win-win.
  - CLI import / export
    - The cmrt keyboard screen allows playing with a single command from the CLI, specifying patches, effects, arpeggios, etc. Aim to enable the same (to the level where it plays just by changing cmrt to cat-plugin-player).
      - Frequent breaking changes are acceptable. Manual migration will be possible later. It's better than being paralyzed by fear of change and halting progress.
  - SMF import / export
    - Currently, outputting SMF with recorded audio plugin names and patch names is implemented; we will further develop this.

## Concept and Goals
- Instrument
  - For example, a synthesizer with a MIDI keyboard plays a sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations required to produce sound.
- Sound Production
  - We aim to maintain "sound production in the author's environment" as much as possible.
- Instant Sound
  - Perceptually, sound plays within 0.2 seconds of app launch. We prioritize the fastest possible playback.
    - Processes like GUI display preparation are deferred until after sound production.
      - Sound playing before the GUI appears could be thought of as a "sound splash screen."
- Educational Use
  - It is oriented more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

## Informal Thoughts Corner
- *Related to the concept.
- I like audio plugin preset sounds.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide an instrument UX.
  - I want to offer an instrument's UX, not a DAW's UX.
- What is an instrument?
  - It is the sum of each layer from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and the performance UI.
        - This is a layer where "vibe coding" can be applied.
        - Building this layer can improve UX.
- An instrument for whom?
  - Cats, babies.
    - Ultimately, that's what I want to aim for, but currently, it's still an instrument for myself.

## Non-Goals
- Robustness
  - Absolutely no bugs, no app crashes, no matter what.
  - Full compatibility. Operates by loading all past settings and data.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the absolute extreme.
- Performance
  - High-speed operation without CPU load in all environments. Sound never cuts out.
- Requests
  - Immediately responding to every user request.

## Miscellaneous
### How to Specify the Main Window's Position and Size at Startup

In `AppData Local/config.toml`, for example, write the following:

```
[window.main]
x = 0.0
y = 0.0
width = 960.0
height = 800.0
```

## License
- MIT