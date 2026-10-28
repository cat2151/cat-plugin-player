# cat-plugin-player

A lightweight application that allows you to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

## Past Challenges and What This App Solves
- Audio plugins sound great!
- To play them, you often need to launch a DAW and perform preparatory steps.
- Lightweight apps for easy playing sometimes required account registration to obtain, or their operation felt "not for me."
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- That's what I decided to do.

## Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

## Demo Video

If you haven't installed any audio plugins, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) for you, as shown here. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

## Features
- Automatic Installation
  - If you haven't installed any audio plugins, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as described above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking a plugin automatically starts playback after the plugin loads.
  - It can also play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit sounds and effects.
- Sounds
  - Each plugin has a variety of preset sounds. You can easily select them using its own sound browser (supported by some plugins only).
  - For plugins not supported by the sound browser, you can simply select sounds using the plugin's GUI.
- Effects
  - Each plugin has a variety of preset effects. You can easily select them using its own effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can simply select effects using the plugin's GUI.

## Future Brainstorming
- * Subject to change based on discretion.
- Leverage strengths not found in [cmrt](https://cat2151.github.io/clap-mml-render-tui/README.ja), such as support for VST3 and GUI.
  - Integrate GUI edit status with cmrt.
    - Load and use edited sounds and effects in cmrt.
  - Experiment with various things using egui.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, playing multiple SMFs together, swapping sounds and effects?
    - Should these be extracted into separate apps? Or is it better to do them in this app because of the sound/effect swapping? What kind of workflow is envisioned?
- Do various things using cmrt's crates.
  - * This also refines the crates, and there are benefits from using them, so it's a win-win.
  - CLI import / export
    - The cmrt keyboard screen allows you to specify patch, effect, arpeggio, etc., from the CLI and play it instantly. Make the same possible (to the level where it plays by just changing `cmrt` to `cat-plugin-player`).
      - Frequent breaking changes are acceptable. Manual migration is possible later. It's better than being unable to move forward due to fear of change, stopping progress.
  - SMF import / export
    - Currently, outputting SMF with audio plugin names and patch names recorded is already implemented; further develop this.

## Concepts and Goals
- Instrument
  - For example, a synthesizer with a MIDI keyboard produces sound when you power it on and press a key.
    - We aim for that level of ease of use.
    - Minimize the operations required to produce sound.
- Playability
  - We aim to ensure it 'just works' in the author's environment as much as possible.
- Immediate Sound
  - Perceived 0.2 seconds from app launch to sound. We've optimized for the fastest possible playback.
    - Processes like GUI display preparation are also handled after the sound.
    - Sound playing before the GUI appears might be considered a sound version of a splash screen.
- Educational Use
  - It is oriented more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - We will make frequent breaking changes.

## Random Thoughts Corner
- * Related to concepts.
- I like the preset sounds of audio plugins.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It is the sum of each layer from audio plugins to the PC keyboard.
    - There are intermediate layers.
      - These are plugin hosts and performance UIs.
      - This is a layer that can be vibe coded.
      - By creating this, UX can be improved.
- An instrument for whom?
  - Cats, babies.
    - In other words, extend the range towards beginners, to the extent that it includes cats and babies (e.g., Android tablets) as targets.
    - * This is purely a metaphor. It is unconfirmed whether it actually works on Android tablets. The likelihood of effort to make it work on Android if it doesn't is also low.

## Out of Scope and What We Don't Aim For
- Robustness
  - Absolutely no bugs no matter what. The app won't crash.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Convenience
  - Pursue convenience and ease of use to the extreme.
- Performance
  - Operates fast without CPU load in any environment. Sound will never drop out.
- Requests
  - Immediately respond to all user requests.

## Miscellaneous
### How to Specify Main Screen Coordinates and Size at Startup

In `AppData Local/settings.toml`, write the following, for example:

```
window = [0, 0, 1800, 900]
```

## License
- MIT