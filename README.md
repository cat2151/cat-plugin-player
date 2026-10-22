# cat-plugin-player

A lightweight application for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd). Written in Rust.

# Challenges So Far and What This App Solves
- Audio plugins offer fantastic sounds!
- To play them, you often need to launch a DAW and perform setup beforehand.
- Lightweight apps for casual playing sometimes required account registration for acquisition, or their user experience felt "not for me".
- Today, if you have a desired UX, you can achieve it through "vibe coding".
- So, I decided to do just that.

# Download

[Download Windows Version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

# Demonstration Video

If you don't have an audio plugin installed, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) like this. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If you don't have an audio plugin installed, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as described above.
- Detection
  - Upon startup, it automatically detects installed plugins.
- Playback
  - Clicking on a plugin loads it and automatically starts playback.
  - It can also play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit patches and effects.
- Patches
  - Each plugin has a variety of preset patches. You can easily select them using a proprietary patch browser (supported for some plugins only).
  - For plugins not supported by the patch browser, you can simply select patches using the plugin's GUI.
- Effects
  - Each plugin also has a variety of preset effects. You can easily select them using a proprietary effect browser (supported for some plugins only).
  - For plugins not supported by the effect browser, you can simply select effects using the plugin's GUI.

※↑This section is written by a human. AI is forbidden from writing here.

# AI Editable Section

Planning to fix readability later. Collapsing for now.

<details><summary>Detailed Features and More</summary>

## Features (Detailed additions by AI)

Oscilloscope display methods and constraints are documented in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- **Detection**
  - In normal startup and editor integration, the complete plugin list and instrument/effect classification are first restored from `%LOCALAPPDATA%\cat-plugin-player\plugin-list-cache.json`.
  - If the cache is missing or unreadable on the first launch, it will be detected and created.
  - After startup, if there are no loads or patch changes and no operation for 2 seconds, a background re-detection is performed.
  - The cache and patch list are updated only if there are changes to the list; otherwise, the display is maintained.
  - If re-detection fails, the existing cache is retained.
  - If plugin detection is successful but no instruments are found, a prompt to install the free Surge XT via winget is displayed.
  - It also displays if only effects are present.
  - If Windows display language is Japanese, it's shown in Japanese; otherwise, in English.
  - Even if `Later` is selected, it will appear every time the app starts as long as no instrument is available.
  - Selecting `View License Agreement` opens the Surge XT license agreement (GPLv3) in a browser.
  - Pressing `Agree to License and Install` executes winget and displays stages and elapsed seconds: '1/4 Preparing', '2/4 Downloading', '3/4 Installing', '4/4 Loading'.
  - If the Windows confirmation screen (User Account Control) appears at 3/4, please select 'Yes'.
  - Upon success, it automatically rescans the plugin list, loads Surge XT (CLAP), and begins playback, similar to `Load`.
  - In this case, the previous instrument/effect restoration is not performed.
  - Even if the prompt is closed, installation continues, and once finished, re-scanning and loading occur.
  - If winget is not present or installation fails, winget's message and manual download links are displayed and logged to `error.log`.
  - Only Surge XT is included, and download progress is not shown.
  - The reason for this choice can be found in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- **GUI**
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, then automatically takes a screenshot, and scales it down to a PNG size that fits the list display frame and screen zoom level, maintaining aspect ratio.
  - It is saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers (ignoring leading/trailing spaces and case) share the same image for both VST3 / CLAP.
  - If name/manufacturer information is incomplete, they are distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated and used as shared images based on list and favorite identification information.
  - Patch state and favorite load destinations are distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing.
  - Within the row, they are scaled to fit the button height, maintaining aspect ratio.
  - Hovering over an image displays it at its saved size.
  - Plugins without a GUI display a placeholder icon of the same height.
  - The GUI prioritizes a position that fits within the screen's work area, aligned with the right side and top edge of the main window.
  - If it doesn't fit, it's placed in the bottom right of the work area of the screen where the main window is located.
  - It is positioned upon display and GUI resize, but does not constantly follow the main window's movement.
  - The GUI is not scaled down; if its width is greater than the screen, it aligns its left edge with the work area; if its height is greater, it aligns its top edge.
  - For blank images or failed captures, retries occur at 1-second intervals for up to 20 seconds from GUI display.
  - Saved blank images are also treated as uncaptured; opening the GUI replaces them with a valid image.
  - After success, it does not recapture.
  - If capture still fails after retrying, an error is displayed.
  - Reopening the GUI retries the capture.
  - GUIs that don't support capture remain with a placeholder icon.
  - If you want to recapture or if a saved PNG is corrupted, delete the shared PNG for the relevant plugin and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are retained during migration).
  - Pressing `?` or `F1` displays an overlay of key operations for that window (main, Browse patches, Browse effects).
  - Functions are color-coded.
  - Close by pressing `?` / `F1` again or `Esc`.
  - Other key operations are disabled while displayed.
  - Pressing `?` during text input will input it as a character.
- **Playback**
  - Surge XT typically waits 100ms for the first phrase to start when audio processing is reconstructed.
  - For CLAP catalog patches allowed by the distribution judgment table, the delay is set to 0ms and linked to that state for subsequent startups.
  - The table is fetched in the background from GitHub and stored in `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-start-delay.json`.
  - It is not bundled; 100ms is used for initial launch, unlisted patches, or mismatch, and the saved table is used if communication fails.
  - Opening the plugin GUI disables this permission because edits cannot be tracked.
  - Closing the GUI and re-selecting a known patch from the browser re-evaluates the delay.
  - 0ms is a decision prioritizing UX; refer to [ADR 0008](docs/adr/0008-surge-xt-automatic-note-start-delay.md) for measurement conditions and limitations.
  - The last played instrument, connected effect(s), Bypass status, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next startup.
  - Playback pattern selection when stopped, and confirmed MML/chord text are also restored.
  - If there's only one instrument candidate in the Plugins list at startup, it's automatically loaded if not already, and playback starts with the selected pattern, even if a stopped state was saved.
  - For multiple candidates, the stopped state is restored.
  - If manually stopped after startup, it remains stopped.
  - Unconfirmed edits are not saved.
  - CLAP Note On/Off messages are sent according to the plugin's supported input format.
  - This also applies to sforzando CLAP, which only accepts MIDI.
  - The scope of the fix and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown.
  - Arrows cycle at both ends of the types; stop is not included in the types.
  - Switching during playback releases all notes of the current phrase and starts from the beginning of the new phrase.
  - When stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop.
  - In the main window, the `Space` key can also toggle start/stop (except during text input).
  - Selection of playback patterns including MML/chords and stop state are restored on next startup.
  - However, if only one instrument candidate exists at startup, it will automatically play from a stopped state.
  - To audition MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods).
  - An example for single notes is `t120 c8r8d4e4r4`, and for chord progressions is `C F G C`.
  - Both are accepted in the same input field, and chord notation conversion is attempted first.
  - Uppercase `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter` / `Confirm`, and canceled with `Esc` / `Cancel`.
  - Cancellation or conversion errors retain the previous valid phrase.
  - In the input screen, `Space` and `i` are used for text input; `Space` for play/stop does not work.
  - Confirmed phrases loop playback with `Play` / `Stop`.
  - It retains rests, note lengths, tempo changes, chords, and trailing rests.
  - Opening the input screen temporarily pauses the overall normal playback.
  - If playback was active just before opening, it plays the new phrase after confirmation, or the original playback pattern from the start after cancellation.
  - If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback.
  - It can also switch to normal playback patterns.
  - Immediately after confirmation, the text and selection/playback state are saved; upon cancellation, the reverted playback state is also saved.
  - It is restored on next startup or from instrument favorites/history.
  - Unconfirmed text is not saved; cancellation or conversion failure retains the previous confirmed text.
  - Changing input or moving the cursor auditions the single note/chord at that position once.
  - Mid-edit or paste operations are also subject to this.
  - Moving within the same articulation unit or simply redrawing does not re-trigger the sound.
  - It reflects the previous tempo, octave, and note length, and does not articulate at rest or command positions.
  - Normal playback stops when editing is opened, and overall playback does not resume after the audition sound ends.
  - The audition sound is released on the next articulation unit, cancellation, confirmation, or instrument change.
  - Auditioning does not change confirmed text or normal selection.
  - For confirmed MML loop playback, the Velocity setting in the `Velocity` field takes precedence over any specified within the input.
  - CC1 can also be controlled from the `CC1 modulation` field.
  - If the playback event capacity is reached with a very dense phrase, sounding notes are released, and articulation stops.
  - You can restart with `Stop` → `Play`.
  - When calling from Favorites/History or changing patches via Browse patches/Browse effects during normal startup, the stop/playback state is maintained.
  - During playback, it plays from the beginning of the phrase and does not carry over any mid-phrase waits from before the change.
  - Leading MML rests are preserved.
  - While the patch browser or effect browser is displayed, `Space` key playback/stop and MML input start in the background are suppressed.
  - Patch changes within a plugin's own GUI are not detected.
  - In SMF selection mode (`--select-smf`), the input track is loop-played with tempo, duration, and velocity.
  - For bulk patch assignment, all tracks are auditioned with the selected patch and the effect of the representative track.
  - Normal session patches, effects, and MML are not restored; only sound snapshots are used for favorites.
  - Patch/effect/favorite/connection changes do not revert to the beginning; playback continues from the stopped position after a brief silence during the swap.
  - MML/normal pattern control fields are not used in this mode.
  - In SMF selection mode with `--master-effects`, the current track's patch/effect are retained for auditioning, and only master effects are edited and returned.
  - The patch browser is not used.
  - Master effects are saved to the conductor-specific meta of SMF format 1, and the results applied to all tracks are confirmed back in the editor.
  - Master support for format 0/2 has not been added.
  - Refer to [ADR 0030](docs/adr/0030-master-effects-selection.md).
  - cat-smf-editor keeps one player resident with `--resident <name>`, bringing the patch/effect browser to the foreground every time `t` / `x` is pressed.
  - The resident player does not load the selected patch/effect itself; instead, it sends the state to the editor, which then plays it with its own instrument.
  - Esc, `q`, close button (×), and Enter to close the effect browser only hide the browser; the player terminates when the editor exits.
  - The previous patch is not restored, nor is the player's normal status.json updated.
  - Role/Preset, Category/Kind, and search strings (including Solo/Mute and exclusion conditions) are saved to the editor's status.json per song, track, and patch/effect; master effects are saved and restored independently per song.
  - It saves even if closed without selecting a patch.
  - Patch list selection is also saved to the editor's status.json per song and track with patch name, instrument, and original patch identification information, and restored after restart.
  - Patch is not reapplied during selection restoration.
  - For Six Sines / TyrellN6 CLAP, when audio processing is reconstructed due to loading, state restoration, or connection changes, it waits for the first phrase to start while continuing normal audio processing to preserve the initial sound.
  - At 48kHz, the wait is approximately 21ms for Six Sines and 85ms for TyrellN6.
  - Leading MML rests are added on top of this.
  - Refer to [ADR 0019](docs/adr/0019-random-patch-all-catalog.md) for application conditions and actual device verification.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` from the same left/right arrows or dropdown.
  - The default value is 100.
  - Range specification increases values in the order of Note On in the phrase, then decreases in the next loop, repeating the action.
  - Changes are applied from the next articulation without restarting the phrase, and are restored on next startup.
  - In the `CC1 modulation` field, you can select `-`, `0`, `127`, `sweep` from the left/right arrows or dropdown.
  - The initial value `-` sends nothing and does not change the instrument's modulation wheel value.
  - `sweep` repeatedly changes from 0 to 127 in 2 seconds, then from 127 to 0 in the next 2 seconds.
  - Dragging the value display bar to the right of the field horizontally sends a value between 0 and 127, incremented/decremented from the value at the start of the drag (it moves 0-127 across the bar's width. If `sweep` is active, it stops, and the field takes that value. Clicking alone does not change it).
  - Selected and dragged values are also restored on next startup.
  - For Surge XT, for each Macro (Macro 1-8) where the current patch uses modulation and the CC number is known, a field like `CC41 Macro 1` appears below the `CC1 modulation` field (numbers are Surge's assignments; default is Macro 1-8 are 41-48).
  - If a patch names a Macro, e.g., `LFO Speed`, that name is displayed instead of `CC41 Macro 1` (hovering over the name shows `CC41 Macro 1`).
  - Selection and dragging the value display bar are the same as CC1; `sweep` repeats with different periods for each Macro and from CC1 (one way from 0 to 127 is 2.9 / 3.7 / 4.3 / 5.3 / 6.1 / 7.1 / 7.9 / 8.9 seconds for Macro 1-8 respectively. CC1 is 2 seconds).
  - The initial value `-` sends nothing and does not change the Macro value saved in the patch.
  - In Surge XT, when `-` is selected, the displayed value is the patch value re-read when the patch is changed or the selection is changed to `-` (moving knobs in Surge XT's GUI does not change this display).
  - Changing the patch via Browse patches, Favorites, History, or Surge XT's GUI reverts CC1 and Macro's `0`, `127`, and dragged values to `-`, playing with the values saved in the new patch (`sweep` continues).
  - The `M` key sets Macro 1-8 to `sweep` collectively; pressing it again when all displayed Macros are `sweep` reverts them all to `-`.
  - Selection is saved like CC1 and carried over to the next startup.
  - Favorite duplicate detection ignores differences in Macro values, same as CC1.
  - Even when changing patches in Surge XT's GUI, the field's visibility toggles immediately.
  - Refer to [ADR 0036](docs/adr/0036-surge-xt-macro-sweep.md) for details.
  - For TyrellN6 (CLAP), when the current patch uses Control A and Control B for modulation, fields like `CC2 Control A` appear in the same location (numbers are assignments saved in the patch for Control A/B).
  - If the patch's Usage field contains a description like `Ctrl A = bass dominator`, that description becomes the heading.
  - Selection, dragging, `M` key, saving, and behavior upon patch change are the same as Surge XT's Macro; `sweep` one way takes 2.9 seconds for Control A and 3.7 seconds for Control B.
  - When `-` is selected, the displayed value is the sent value; patch values cannot be retrieved.
  - Changing patches in TyrellN6's GUI does not switch the fields; they switch when patches are changed via Browse patches, Favorites, or History.
  - Macros for other plugins are not supported.
  - Refer to [ADR 0038](docs/adr/0038-tyrell-n6-control-ab-sweep.md) for details.
  - Guitar open strings (MIDI note numbers 40, 45, 50, 55, 59, 64 - E, A, D, G, B, E) trigger Note On sequentially at 125ms intervals.
  - Two seconds after the last Note On, all six notes simultaneously Note Off, then repeat after 0.5 seconds.
  - On successful `Load`, it is memorized when selecting `Sequence` type, starting, or stopping.
  - The memory remains after stopping or `Remove`.
  - Old history/favorites are restored with the conventional 4-note pattern.
  - Upon normal shutdown and `Remove`, the binary state of patches, parameters, etc., exposed by plugins via UAPMD's state API is saved to `%LOCALAPPDATA%\cat-plugin-player\states\` per plugin.
  - On next startup or reloading the same plugin, it is restored before playback begins.
  - Instrument `Load` replaces only the instrument, retaining connected effects and settings.
  - It saves the state of the plugin to be replaced, and switches only after successfully creating, restoring, and connecting the new plugin.
  - If it fails, the original configuration is maintained, and an error is displayed.
  - Only when reloading the minimal Surge XT configuration at startup to the same instrument, the same preservation, regeneration, and recovery path as `Show UI` below is used.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation.
  - UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state.
  - It does not include duplicating external samples or saving during forced termination.
  - At startup, it reads the saved list of all plugins, directly loads the previous instrument and effect, restores their states, and starts playback.
  - Afterward, the GUI is initialized, and once operations settle, the list is re-detected in the background.
  - If there's no full list cache on the first launch, it detects after GUI display.
  - If the previous instrument was Surge XT (CLAP / VST3), only this one instance is created at startup in a configuration that omits patch, wavetable, and FX preset lists, to enable faster playback.
  - In this minimal configuration, Surge's GUI menus are almost empty; pressing `Show UI` for that Surge rebuilds it in a normal configuration, preserving the latest patch, edits, Favorite selection, playback settings, and connected effects, then opens the GUI.
  - Sound temporarily stops during this process.
  - If it cannot be saved, it does not rebuild and retains the original instrument.
  - If creation, restoration, or connection fails, it attempts to recover once from the saved content and displays an error.
  - If recovery also fails, it stops, leaving the saved content; please retry with `Load`.
  - Repeated presses do not increase rebuilding attempts.
  - Surge XT created via `Load`, patch browser, Favorites, History, or effects uses a normal configuration.
  - It can be disabled with the environment variable `CMRT_SURGE_MINIMAL_DATA_HOME=0`.
  - Refer to [ADR 0034](docs/adr/0034-surge-xt-startup-minimal-data-home.md) for reasons, measured wait times, and verification scope.
  - If direct loading is not possible due to old history or file movement, it is restored from the scan results after GUI display.
  - Old history is updated once played.
  - If the main instrument is CLAP Floe, when audio processing is reconstructed due to loading or patch restoration, the start of the first phrase is delayed by 100ms.
  - Leading MML rests are added on top of this.
  - This is a temporary measure for observing the symptom of missing initial sounds, and its resolution on actual devices is unconfirmed.
  - Refer to [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - For the adoption history of startup acceleration, bugs encountered with Show UI, and changes in disposal order, refer to [ADR 0034](docs/adr/0034-surge-xt-startup-minimal-data-home.md); for measurement methods, see [Developer's Guide](#measuring-startup-time).
- **Effect Routing**
  - Opening `Browse effects` / `E` / `X` automatically loads the corresponding CLAP effect if there's only one candidate, making that slot the target for preset changes.
  - If already connected, it uses that slot without reloading.
  - If no instrument is loaded and there's only one instrument candidate, it first loads the instrument and starts playback.
  - If there are multiple instrument candidates, please select an instrument first.
  - The number of candidates is determined according to the CLAP priority setting in the Plugins list, and search strings do not reduce the number of candidates.
  - It is a serial connection: one instrument → multiple effects → output.
  - Load an instrument, then press `Load` for the first effect, and subsequently add more with `Add` to the end.
  - Connected effects are removed one by one with `Remove`.
  - Duplicate connections of effects with the same format + ID are not allowed.
  - Changing effects in Browse effects requires an instrument.
  - Candidates are retrieved from default effect/preset locations explored by the play-server shared core, without substituting for VST3.
  - During preparation, current playback continues, and loading, scanning, and patch operations are disabled.
  - In normal startup, it remains stopped if stopped, or restarts the current phrase from the beginning if playing, maintaining MML, Velocity, CC1 global selection, and other effects' order, settings, and Bypass state.
  - On failure of creation, application, or connection, it reverts to the original configuration.
  - After success, the applied state and connection are saved for the next startup, and are not added to Favorites/History.
  - Save failure is displayed as already applied.
  - Using `Browse effects` next to `Sequence` or the `E` / `X` key opens a separate, maximized effect browser window, allowing you to select presets from a list.
  - It cannot be opened while editing MML.
  - Filter by Category → Kind and search on the left.
  - Classification and selection targets are the same as cmrt's effect selector.
  - Search is case-insensitive regular expression, whitespace AND, exclusion by `-condition`, applying to preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`).
  - Invalid regular expressions during input display an error and retain the previous list.
  - The top row shows Search, Target Slot, `Random`; `/` inside the browser moves input to the search field; `R` randomly selects from filtered results; `W` toggles column width between fitting all preset names and default width.
  - Multi-column display of the list, movement with up/down/left/right keys / `h` / `j` / `k` / `l` and numeric repetitions, `PageUp` / `PageDown` movement, and `Enter` / `Esc` to close are the same as Browse patches.
  - In results with mixed effects, effect names are attached per effect; if Kind differs, preset names are color-coded per Kind.
  - Names that don't fit in a cell always retain the trailing preset name (part excluding folder), truncate the folder name/effect name to its first character + `…` to its left, and display longer parts closer to the preset name.
  - Full names can be seen by hovering the mouse.
  - The target slot is selected from `Add new effect` or `Replace n` of a connected effect.
  - When opened, it defaults to the slot last written by this browser, otherwise the last effect, or `Add new effect` if no effects are present.
  - Applying with `Add new effect` makes the added effect the target; subsequent selections replace that slot, so the number of effects does not increase by continuous selection.
  - If the target slot is removed, the last effect becomes the target.
  - Presets for effects connected to non-target slots are shown in gray; selecting them displays a reason and does not change them.
  - Presets applied to the target slot via this browser are marked with `▶`, and opening it selects them (they are not reapplied).
  - When a different candidate is selected by click, key movement, or filtering, an immediate application is requested.
  - Simply opening does not change the effect.
  - You can select the next even during preparation; uninitiated selections are replaced by the latest one, and closing cancels them.
  - Category, Kind, search string, list selection position, and target slot are saved on exit and restored on next startup.
  - Key movement starts from the restored selection.
  - Category/Kind are restored by name; if they no longer exist, they revert to `ALL` in their respective fields.
  - Selection is restored by preset; if it no longer exists in the catalog, it becomes unselected, and key movement starts from the beginning.
  - The target slot is restored by its position in the chain; if no effect is at that position, it becomes the default slot when opened.
  - If the effect before or after replacement in Browse effects is CLAP Surge XT Effects, the sound immediately before the switch fades out over approximately 50ms, and the sound after the switch fades in over approximately 50ms.
  - This is a temporary measure to suppress a loud 'pop' sound caused by the instrument's lingering sound passing through the new effect (especially distortion types).
  - The switch is delayed by that amount (max approx. 70ms), and lingering sounds might be faintly audible during the fade-in.
  - Fading does not occur for switches between other effects or changes in patches/instruments.
  - Refer to [ADR 0027](docs/adr/0027-surge-xt-effect-swap-fade.md) for reasons and scope of application.
  - For CLAP Surge XT Effects presets, instead of temporarily launching the plugin for preparation, a pre-prepared state table is fetched from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) on each startup (saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-fx-states.json`).
  - This is to avoid a bug where the app crashes if presets are continuously selected.
  - If it cannot be fetched, the last saved table is used; if that's also unavailable, Surge XT Effects presets do not appear as effect browser candidates, and a reason is displayed.
  - The table is created from Surge XT 1.3.4 presets; presets with different content (e.g., different Surge XT versions) are excluded from candidates, and the number of excluded items is displayed.
  - Refer to [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for reasons.
  - The connection order is displayed in `Routing`.
  - You can change the order by dragging an effect's `↕` button up or down.
  - While dragging, the moving effect name and insertion line are displayed.
  - Instruments are not subject to reordering.
  - You can edit patches and effects with `Show UI` for each plugin.
  - `Instrument` / `Effect` are displayed with the same type color as the list on the left.
  - Removing all effects reverts to a direct connection from instrument to output.
  - `Bypass effect` targets the entire chain, allowing audio to pass through while retaining the instance and settings.
  - It can also be toggled with the `b` key (does not work in MML input screen, while Browse patches/Browse effects is displayed, or during text input).
  - During Bypass, connected effects in Routing are displayed in gray, with `Bypassed` appended.
  - `Sequence` is for instruments.
  - Even if Sequence is stopped, audio processing continues, allowing effect tails to decay.
  - On next startup, it restores the instrument, order of all effects, their respective states, and Bypass status before playback.
  - History of one old-format effect can also be loaded.
  - If effect restoration fails, an error is displayed, it reverts to a direct instrument connection, and the saved configuration is retained.
  - Addition, removal, and reordering are saved only if preparation of the new connection succeeds.
  - If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output.
  - Mono → stereo is duplication; stereo → mono is the average of left/right.
  - Auxiliary input is silent.
  - There may be short audio dropouts when changing connections or Bypass, and sounding notes and tails might be cut off.
  - Sidechain, parallel connection, and host-side Dry/Wet are not supported.
- **Patches**
  - If no instrument is loaded and there's only one instrument candidate, `Browse patches` / `T` automatically loads it and starts playback.
  - Requests during loading/scanning are processed after completion; if the list is closed, uninitiated automatic loads are canceled.
  - If there are multiple candidates, it does not select automatically.
  - The number of candidates is determined according to the CLAP priority setting in the Plugins list, and search strings do not reduce the number of candidates.
  - Load failures are displayed, and no automatic retries are made.
  - Supported formats and catalog limitations for the patch list itself are as before.
  - Using `Browse patches` next to `Sequence` or the `T` key opens a separate, maximized patch browser window on the same screen as the main window.
  - The main window's position and size are not changed.
  - It cannot be opened while editing MML.
  - Filter by framed Role → Preset → search.
  - Roles/Presets with no patches are not displayed.
  - Items that result in 0 entries after Solo/Mute filtering (e.g., `FM Bass` when soloing anything but Dexed) are also hidden.
  - Search string does not hide; custom conditions are displayed even if 0 items, so they can be deleted.
  - The left condition area and right Patches list are individually scrollable.
  - The list uses the remaining width and height, allowing operation even when unmaximized or in a narrow window.
  - The top row shows Search, `Solo/Mute`, `Random`, `☰`.
  - The search field uses the remaining width; the `/` key inside the browser moves input to the search field.
  - Instrument condition details can be checked by hovering over `Solo/Mute`.
  - Clicking this entire button or pressing the `M` key inside the browser opens a vertical Solo/Mute overlay, confirming all selections with `OK` or `Enter`.
  - Solo/Mute are arranged directly to the right of the instrument name.
  - Re-clicking the selected mode reverts to normal; `Esc` / clicking outside the overlay discards changes.
  - Multiple Solos are treated as OR; Mute is treated as exclusion.
  - Search uses case-insensitive regular expressions, whitespace AND, exclusion by `-condition`, and `plugin:` conditions (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`).
  - Aliases of integrated patches can also be searched.
  - Invalid regular expressions result in an error and stop application, random selection, and condition saving.
  - If 0 items, conditions can be re-evaluated by editing the search field or changing Role/Preset.
  - Catalog load pending/failure is displayed separately from 0 items.
  - Role, Preset, and search string are saved on exit and restored on next startup.
  - Preset is restored by name; if the condition was deleted, it reverts to `ALL` for that Role.
  - If the selected Role results in 0 items during restoration or Solo/Mute change, it reverts to global `ALL`; if Preset results in 0 items, it reverts to `ALL` for that Role.
  - The list displays single-line, non-wrapping patch name cells arranged in multiple columns (left → right → next row) depending on width.
  - It reverts to 1 column in narrow windows.
  - Resizing retains the selected patch; merely changing the number of columns does not reload.
  - Clicking a patch does not move the list's scroll position.
  - Up/down/left/right keys or `h` / `j` / `k` / `l` move as per visual arrangement.
  - Left/right move to previous/next patches (to the end of the adjacent row at row ends); up/down move to adjacent rows; if there's no patch directly below in a short final row, it moves to the end of that row.
  - Typing a number first, like `2j` or `10k` (vim-style), moves that many times (stops at ends; numbers are canceled by non-movement keys).
  - `PageUp` / `PageDown` move by 10 steps up/down; combined with a number, they move by that multiple.
  - Key movement, Random, filtering, and column count changes scroll only if the selected row is off-screen, making it visible.
  - It does not scroll if all patches fit.
  - In results from a single instrument, the instrument name is omitted; in mixed results, it is retained.
  - Text color indicates only one difference within the results.
  - If instruments are mixed, the instrument name portion is color-coded per instrument.
  - For a single instrument, when Preset is `ALL`, patch names are color-coded per relevant Preset, or per category if that doesn't distinguish them.
  - Colors are Monokai-based.
  - If all are the same, it's normal text color.
  - Colors repeat in a cycle of 5, so distant groups might have the same color.
  - For pink patches, the patch name 'pink' is prioritized.
  - Classification folders, etc., are omitted; if same-named patches need distinction, necessary paths are retained.
  - Dexed program assignments are maintained.
  - Hovering the mouse shows the full display path, original application path, active category, and number of merged items (2 or more).
  - Load/Size are not permanently displayed in the list, nor are `no category` / `1 merged` shown.
  - For normal patches, changing selection via mouse/key movement immediately requests application; the same behavior occurs when a different candidate is chosen due to condition changes.
  - Simply opening does not change the patch.
  - Even if the list's selection position is lost due to filtering, if the currently playing patch remains in the results, selection moves to that patch and it is not reapplied.
  - Upon opening, it restores the saved list selection using patch name, instrument, and original patch identification information, and scrolls to make it visible (it is not reapplied).
  - Row numbers are not saved; it selects the same patch even if the catalog order changes.
  - If no list selection record exists, if a patch currently playing (applied via Browse patches) is in the current results, that patch is selected.
  - The playing patch is saved on exit and inherited as the restored patch on next startup.
  - Adding/deleting effects or using Browse effects preserves this inheritance.
  - If a saved patch is deleted or excluded by filtering, it becomes unselected, and the patch is not changed.
  - If neither list selection record nor playing patch information exists, it is unselected.
  - After changing patches via Favorites/History, instrument loading or deletion, the playing patch is unknown, and selection is cleared.
  - Key movement starts from the selection upon opening; if unselected, it moves to the beginning.
  - Next selection is possible even during preparation; uninitiated requests are replaced by the latest one.
  - Upon successful normal application, it does not display duplicate patch names or fixed margins at the bottom.
  - A brief notice is displayed only if the selected and applied patches differ, indicating that the previous patch might continue sounding for unconfirmed 'pink' patches, etc.
  - Pink Space prompt, application failure, and save errors are displayed.
  - Pink patches with sample capacity of 64,000,000 bytes or more are not automatically applied (unknown capacity is treated as normal).
  - After selection, `Space` opens an instrument/patch/capacity confirmation overlay; pressing `Enter` within it initiates preparation/application.
  - During confirmation, `Esc` cancels without making changes.
  - Even patches applied once require re-confirmation.
  - During loading, a progress overlay is displayed, and cancellation midway is not possible.
  - Space/Enter in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is equivalent to `OK`).
  - The `Random` button or `R` key in the browser selects uniformly from the displayed results.
  - During text input, `T` / `R` / `M` / `/` / `h` / `j` / `k` / `l` / numbers / `Enter` shortcuts do not work, nor do `R` / `M` while an overlay or save menu is displayed within the browser.
  - For pink patches, selection alone is not enough; the same Space → Enter confirmation is required.
  - In normal startup, `Esc`, `Enter`, or OS close of the browser window closes only the browser, retaining the main window and the last successfully applied patch, discarding uninitiated requests and old preparation results.
  - Processes that have already begun application are completed.
  - When confirmation overlay, Solo/Mute, or save menu is open, `Esc` / clicking outside the overlay closes only the child, and does not pass the same input to the browser or background operations.
  - OS close closes the entire browser.
  - It can be reopened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to a Role via `Save condition to Role` in the hamburger `☰` menu, and deleted from a Preset via `Delete`.
  - The destination for adding from ALL is Etc.
  - It is handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` within cat's config directory.
  - On load corruption, the original file is protected, writing is stopped, and save failures are also displayed on screen.
  - During the same app session, Role/Preset/search/Solo/Mute are retained, but only saved custom conditions are restored after restart.
  - TUI conditions and Favorites are not shared/migrated.
  - During Browse patches preparation, current playback continues, and conflicting load, scan, and patch operations are disabled.
  - It may wait for sample loading, such as with Floe.
  - It retains MML, Velocity, CC1 `sweep` / `-` selections, effect order, state, and Bypass (CC1 and Macro `0` / `127` revert to `-`), and saves the state for next startup after success.
  - Favorites/History are not automatically added.
  - Application success and save failure are displayed distinctly.
  - Preparation/application progress updates even when a child window is in focus.
  - Patch browser candidates are CLAP patches from Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, Vaporizer2 in the existing cmrt catalog, matching installed instruments by format + ID.
  - The patch browser catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode.
  - VST3 substitution and unknown instruments are not supported.
  - Effect preset selection uses the Browse effects field in the effect routing section.
  - Dexed retains program assignments within its cartridge.
  - Floe/sforzando external samples are required in their original location; there is no function to duplicate or save samples.
  - In environments without cmrt or the `catalog.json` mentioned above, Surge XT's patch list is fetched from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) on each startup, saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json`, and used as candidates for Browse patches.
  - If it cannot be fetched, the last saved list is used; if that's also unavailable, the patch browser displays a load failure.
  - Until fetching is complete (up to 10 seconds), a 'loading' message is displayed.
  - The list is created from Surge XT 1.3.4; for different Surge XT versions, added patches won't appear, and selecting a non-existent patch displays a preparation failure.
  - If `catalog.json` exists but is unreadable, it does not switch.
  - The reason can be found in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - If preparation/application fails, a reason is displayed.
  - It restores the original configuration, state, and playback state; instances where state restoration also failed are prevented from saving.
  - Save failure after patch application is displayed as 'already applied'.
  - Instrument-specific preparation/application conditions and verification scope are documented in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - Using `★ Add favorite` for each plugin allows saving multiple patches/settings at that moment as favorites.
  - In the main screen and Browse patches, the `F` key saves the currently loaded instrument; in Browse effects, it saves the effect connected to the target slot (unapplied candidates are not saved).
  - It does not work during text input or while a dialog is displayed, nor can it save during loading/preparation.
  - For instruments, it also saves the playback pattern, selection while stopped, and confirmed MML/chord text.
  - For effects, playback settings are not saved.
  - Names for new items prioritize the stem derived from the recorded patch/preset name (excluding path and corresponding preset file extension) for both instruments and effects, using the format `stem sequence_number`.
  - Effect plugin name prefixes are also removed.
  - Dots and program numbers in patch names are retained (e.g., `…/TX7-34.SYX/22 <Flute.22>` → `22 <Flute.22> 1`).
  - Full names are retained for search/hover display.
  - If the name is not retrieved or is blank, it defaults to `Plugin Name sequence_number`.
  - It can be changed later.
  - Existing items are not renamed; re-saving the same state retains the existing name.
  - When a different plugin is `Load`ed, the patch, settings, and playback pattern of the replaced plugin are automatically recorded to History before the switch.
  - It is not added for reloading the same plugin or restoration at startup.
  - Confirmed MML/chord text and selection while stopped for instruments are also recorded within the same scope as favorites.
  - For effects, playback settings are not recorded.
  - If saving fails, the switch is aborted.
  - During manual/automatic saving, it checks for duplicates of the same plugin, role, playback pattern, confirmed MML text, and state, reusing existing items and names.
  - History's automatic recording treats differences only in stopping as identical, but MML with different text is retained as a separate item.
  - Effects do not compare playback settings.
  - Exceptions, reasons, scope, and limitations for each product can be found in [Plugin-specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md).
  - In the verified state format for TyrellN6 CLAP, the compressed portion that changes only with playback is excluded from duplicate detection, and patches with identical text-based patch settings are grouped.
  - Verified PCore `UI_op=9/10` and trailing empty lines in text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)).
  - In the verified format for Vaporizer2 CLAP, tiny differences in MSEG time/coordinates (absolute difference for specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less; see [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) that arise during loading/saving are treated as identical.
  - Edits within this tolerance cannot be distinguished.
  - Saved data is retained as is.
  - Switch lists using the `Favorites` / `History` / `Plugins` tabs in the center.
  - Favorites maintain manual order; new additions are placed at the top.
  - Order is not changed during recall, resaving, or recording to History.
  - Filterable by favorite name, plugin name, format, plus the preset name of the saved patch/effect (case-insensitive partial match).
  - Names successfully applied via Browse patches/Browse effects are separately recorded and displayed as supplementary information in Favorites/History.
  - It is carried over after restart and favorite recall.
  - Name changes/preset changes within the plugin GUI are not tracked; the name last applied by the browser is retained.
  - Existing unnamed items can be searched with conventional information; applying and resaving the same patch from the browser completes the name.
  - The `/` key in the main screen shifts focus to the filter field (common to all tabs; does not work during text input, MML input screen, browser, or dialog display).
  - Turning ON `Reorder favorites` in the Favorites hamburger menu allows sorting with `↑` / `↓` at the end of the row, saving instantly.
  - Row click loading / effect removal behave as before.
  - `Sort by plugin name` sorts and saves once in ascending order of plugin name (case-insensitive, preserving relative order for same-named items).
  - Manual adjustments can still be made afterward.
  - Sorting is disabled during filtering; sorting mode is OFF at startup.
  - Refer to [Adoption History and Specifications (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is in order of most recent registration.
  - If deemed the same patch, it reuses the saved patch, updates the registration time, and moves it to the top.
  - Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days).
  - Manual Favorites are maintained.
  - All old-format items are migrated to History, and items not starting with `auto ` are also kept in Favorites (because old formats do not record the origin of auto-saves, ensuring all renamed auto-saves are included in History).
  - Refer to [Policy and Reasons (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus.
  - Renaming/deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default.
  - If there are CLAP plugins with the same name, manufacturer, and type, VST3s in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are displayed).
  - Turning it OFF displays both formats.
  - The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on next startup.
  - Favorites and the format for loading/restoring are not changed.
  - In both tabs, clicking the name/image loads it, and clicking again unloads a connected effect.
  - The currently loaded/connected row is colored, and the color is maintained even when effects are bypassed.
  - The same operations can be performed with the `Load` / `Add` / `Remove` buttons in Plugins.
  - Favorites are displayed one item per line.
  - Long names/supplementary information are abbreviated; hovering the mouse shows the full text.
  - Waveform and spectrum are displayed below Routing on the right; the width of the right side can be adjusted by dragging the boundary.
  - With `On right` in `Audio analysis`, the analysis display can be toggled between bottom-right / bottom-screen (bottom-right at startup).
  - In sforzando CLAP's verified state format, it reuses existing favorites rather than creating new ones for identical patches, considering only differences in save counter and played Note/CC1 values.
  - Differences in patch settings are retained.
  - Refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for application scope and limitations.
  - Each row displays a colored `Instrument` / `Effect` on the left.
  - Clicking a favorite name once restores its patch/settings.
  - In normal startup, it restores the confirmed text and playback pattern selection saved from instrument favorites.
  - In SMF selection mode, it uses only the patch state and retains the input SMF.
  - While stopped, it remains stopped; `Play` plays the restored phrase.
  - During playback, it switches to the saved pattern/MML.
  - Favorites without old-format text retain the current text; old-format stopped items retain the current selection.
  - If conversion or state loading/connection fails, it recovers the original text, selection, and playback state, and does not save the failed content to the session.
  - Instrument favorites maintain connected effects; effect favorites maintain instruments and MML.
  - An instrument is required to recall an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed, and re-clicking the name removes that effect.
  - The same behavior occurs if settings are edited after recall or during Bypass.
  - The instrument, other effects, and saved favorites remain.
  - A different patch with the same format + ID switches settings at the current position; unconnected effects are added to the end.
  - The selected favorite for each effect is also restored on next startup.
  - Rename/delete can be done from `…` at the end of the row.
  - Edits after recall or conventional auto-saves do not overwrite favorites.
  - `Last favorite` indicates the last saved/recalled favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`.
  - There may be short audio dropouts during saving/restoring, and the savable scope is the same as conventional state saving.
- **Lissajous Display**
  - `Lissajous` is displayed below the oscilloscope in the right analysis pane.
  - Disabling `☰` → `On right` in the oscilloscope arranges Spectrum, Oscilloscope, and Lissajous horizontally at the bottom of the screen.
  - A 45° rotated Lissajous displays the most recent 50ms of output (max 4096 samples), reflecting effects and Bypass.
  - Common automatic scaling for left/right maintains volume differences and aspect ratio of the figure; in-phase at same level becomes a vertical line, out-of-phase a horizontal line, sine waves with phase difference become ellipses or circles.
  - Left-only output results in a top-left to bottom-right diagonal line; right-only output, top-right to bottom-left; mono, a vertical line; silence, a central dot.
  - It updates independently of notes, cycle count, or trigger settings; titles can be displayed with `Show analysis labels`.
  - Axes and drawing methods can also be checked in the graph's tooltip.
  - Directly below Lissajous, `Correlation` displays the left/right correlation over the last 150ms as a bar and numerical value from -1 to +1.
  - +1 indicates in-phase, near 0 indicates weak correlation, and -1 indicates out-of-phase.
  - Negative values are shown in red and serve as a guideline for judging cancellation when mixed to mono.
  - If silent or one side is silent, it displays '—'.
- **Waveform Display**
  - The `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) outputs, reflecting effects and Bypass, superimposed.
  - Mono displays the same waveform on both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and `1`, `2`, `4`, `8 cycles` can be selected.
  - After Note Off, tails are displayed based on the last note.
  - Outputs including chords or detuning may not perfectly repeat within that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel.
  - If the left channel is silent, it uses the right channel; if no crossing, it displays from the beginning.
  - `Similarity` sweeps the buffer from the beginning for the display width, 1 sample at a time, to find the position with the maximum correlation coefficient to the previous display.
  - In case of a tie, the earlier position is adopted, and the same start position is used for left and right.
  - For the first display or if the comparison target is silent, zero cross is used.
  - Comparison history is reset when note articulation, playback pattern, display cycle count, trigger method, or audio stream changes.
  - A waiting display is shown until sufficient data is collected.
  - Exploration is performed in a separate thread from audio processing.
  - Display updates may be slower for lower notes or more cycles.
  - If display data is interrupted, the history is reset.
  - Default display settings at startup are 4 cycles and zero-cross.
  - Display settings are not saved.

## Settings: Window Position and Size

You can specify the main window's startup position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit after closing the application. Session and favorite selection states are saved to `status.json` in the same directory; normal saving does not overwrite the TOML. States mixed in old TOML are migrated on first launch, the original file is saved as `config.toml.before-status-migration.bak`, and then only state items are removed. Comments, formatting, and unknown items in settings are retained, and independent comments attached to deleted items are left at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates, used as the startup position if both are specified. Omitting them reverts to the conventional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. Either can be specified; omitted values or values less than or equal to 0, or non-finite values, revert to 900 width / 600 height. Changes are reflected on next startup. The plugin placement method is currently `right_then_bottom_right` only, with the same behavior if omitted. GUI size and placement are calculated based on the actual window frame, screen work area, and DPI. Automatic saving of manually moved main window/GUI positions, and per-plugin position specification, are not performed.

## Local Build and Run

Clone this application, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, and run from this application's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` are always locally referenced from Cargo.toml. play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to the conventional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` requires their re-export. The patch browser uses the public Role/Preset/filter API and tui-core's sample capacity detection. Browse effects utilizes play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions, and tui-core's search conditions. Missing checkouts or insufficient APIs result in build errors; no automatic switching to Git versions is performed.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in a sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The UAPMD location during build is specified by an environment variable. `config.toml` is not read as build settings.

Do not use `cargo install`; directly execute the generated exe in the default `target/release`. Building uses local code (including uncommitted changes) and the existing `target/shim`, but source changes may require reconstruction and rebuilding. If `target/shim` was created with a different version like VS 2022, it will continue to build with that version; therefore, delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe, extracted to the cache directory, and loaded when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for 3-repo dependency configuration and operational policy. TUI local switching is not required for building cat. If TUI itself is cross-built, use TUI's existing `python scripts/cross_repo_local.py on` procedure; off is handled manually. Local references also use uncommitted changes; Cargo.lock does not fix the content of each checkout. During verification, record the HEAD and diffs for each repo.

The first-time introduction of Surge XT is consolidated in `shared/first_instrument_install.rs`; cat-smf-editor also references the same instrument absence detection, license screen, and winget processing. Each app is responsible for rescanning and loading after successful installation.

Host Rust/native and SMF processing are consolidated in `shared/`; cat-smf-editor also references the same canonical module. No independent crates are added; each app bundles its own DLL from the default target/shim.

The common host supports direct parameter automation for CLAP plain values and VST3 normalized values. The UI enumerates parameters, retrieves their value representations, and passes resolved targets in fixed-capacity batches to audio processing. The editor's dedicated track utilizes this path. Conditions for generation/fix, reapplication, regression testing, and unsent author reports are in [Parameter Automation Fix](patches/uapmd/parameter-automation/README.md); the reason for adoption is in [ADR 0037](docs/adr/0037-direct-parameter-automation.md).

Starts the normal GUI without arguments. `--help` and `--version` are also available.

SMF selection mode is `--select-smf <SMF> --browser patch|effect --preserve-position`. If browser is omitted, it defaults to patch; position preservation requires explicit specification. Passing `--patch-filter <program_name>` searches the patch browser by that name from ALL conditions. Regular expression symbols in the name are treated as literal characters; whitespace implies AND. Only if 0 items are found after catalog loading, it switches once to an OR search with words joined by `|` (e.g., `church organ` → `church|organ`). With `--patch-filter-file <JSON>`, search, Role, and Preset are restored as is, and returned to the same file upon successful exit. If no file exists, initial search is used; existing file conditions take precedence. Empty searches are also retained, and automatic switching to word OR is not performed for restoration conditions. Not used for effect/master selection. The search field can be edited as a case-insensitive regular expression as usual. With `--preview-smf <SMF>`, you can specify a separate SMF for all-track auditioning, distinct from the return destination. All track MIDI events are sent to the selected instrument/effect, and format 2 reflects per-track tempo. The audition SMF is not overwritten. With `--start-seconds <seconds>`, you can specify the initial audition position; with `--loop-end-seconds <seconds>`, the loop end beyond the input SMF. If omitted, it's from the beginning to the end of the input SMF. Use finite, non-negative seconds; the start position must be before the loop end. Controllers and sustained notes at the start are restored, and natural loop boundaries revert to the beginning. Original SMF events are not changed. Adding `--browser-only` opens the browser in the foreground without displaying the main screen. This option is specific to selection mode. In normal track selection, buttons within the browser can navigate to other browsers. With `--master-effects`, it starts from the effect browser and does not proceed to the patch browser. The main screen can be displayed from `Favorites / routing` only when favorites/routing are needed. Enter to close the browser is also treated as saving and exiting the entire player. On save failure or during change processing, the browser is retained, and errors are displayed within the browser. Esc, `q` not used for search input, and main window/browser close button (×) save the instrument, ordered effects, and bypass states to the input SMF and exit. With `--master-effects`, only the conductor's master effect/bypass are saved, retaining track settings. In the search field, `q` is character input. During change processing, it waits for confirmation; on save failure, it retains the screen and selection, allowing retry. Normal status.json is not updated. Existing state saving/favorite operations during selection use the same save destination as usual. State compatibility across OS/plugin versions or external sample duplication is not guaranteed. Refer to [Saving Contract and Shared Host (ADR 0029)](docs/adr/0029-shared-host-smf-selection.md).

## For Developers

### Offline WAV Acquisition

`--render-wav` generates a new host and instrument for each call, loads the specified binary state, acquires a 200ms initial sound at note 48, velocity 100, CC1=0, and then exits. It does not use GUI, audio device, external effects, or normal session restoration.

```powershell
& ./target/release/cat-plugin-player.exe --render-wav X:/measure/out.wav --plugin-format CLAP --plugin-id org.surge-synth-team.surge-xt --plugin-path 'C:/Program Files/Common Files/CLAP/Surge Synth Team/Surge XT.clap' --state-file X:/measure/state.bin --wait-ms 10
```

Format is `CLAP` or `VST3`; id and path are specified to match the plugin that saved the state. Create the parent folder for the output destination first, and use a new name for the WAV that does not have an existing `.json` with the same name. Input state, Favorites, History, and normal config/status/state are not overwritten.

Saves 9600 frames from note on as a 48kHz, 256 frames, stereo, 32-bit float WAV. `--wait-ms` is a non-negative integer audio sample time. During this time, it processes empty events and renders without syncing to real-time, even after articulation. Normal startup's automatic wait is not added. The JSON records SHA-256 of state/PCM, plugin identification, wait, articulation position/processed frames, silence detection, and time taken for generation, state application, audio processing, and saving. Time spent waiting for generation callback is a separate item. Recordings with all samples as 0 exit non-zero with `status=silent`, leaving WAV/JSON; load/processing errors also exit non-zero. Even with the same input, PCM may not be identical across plugins.

### Surge XT CLAP Initial Sound Wait Judgment Table

All catalog patches are measured once each condition in a new process/instance for fast offline measurement. Normal config/status/state, Favorites, History are not overwritten. WAVs, states, and logs are left in the measurement-specific run directory within the cache; the distribution JSON saves start positions, errors, and 0ms adoption decisions for all patches.

```powershell
& ./target/release/cat-plugin-player.exe measure-surge-start-delay --output ../cat-music-patterns/surge-xt-start-delay.json
```

Defaults are 10ms vs 100ms, 200ms recording, and 0ms permission for start position difference within 1ms. Comparison wait can be changed with `--candidate-wait-ms 0`, etc.; judgment conditions can be changed with `--reference-wait-ms`, `--max-onset-delta-ms`, `--onset-ratio`. 0ms permission is an adoption policy; 10ms vs 100ms results are not treated as actual 0ms measurements. `--jobs` is concurrency (default 4), `--timeout-seconds` is render limit for each (default 30s), `--limit` is item count restriction for operational check, `--work-dir` is parent directory for saving measurement runs. Silence/errors are categorized separately, and processing continues.

Re-rendering is not required if only thresholds are changed. You can re-select, for instance, with a 2ms tolerance, while retaining original measurement conditions and values.

```powershell
& ./target/release/cat-plugin-player.exe measure-surge-start-delay --reclassify ../cat-music-patterns/surge-xt-start-delay.json --max-onset-delta-ms 2 --output X:/measure/reclassified.json
```

The output table is replaced only upon successful completion. Committing/pushing to cat-music-patterns is performed as a separately explicit operation. It does not wait for communication to complete at startup, and table updates do not restart playback. Refer to [Measurement Key Points and Adoption Decision (ADR 0008)](docs/adr/0008-surge-xt-automatic-note-start-delay.md).

### Measuring Startup Time

Specifying an output destination for `CAT_STARTUP_TRACE` records each point in the startup process to CSV. If not specified, it does not record and does not change startup order or auto-playback conditions.
Example of running in the root of a Release-built repository:

```powershell
$env:CAT_STARTUP_TRACE = Join-Path (Get-Location) 'target/startup.csv'
try {
    & ./target/release/cat-plugin-player.exe
} finally {
    Remove-Item Env:CAT_STARTUP_TRACE
}
Get-Content ./target/startup.csv
```

Create the parent directory for the output destination beforehand. Files with the same name are overwritten. Each measurement point is recorded only once per process, so restart the app each time for comparison.

- The baseline is the entry point of Rust's `main()`, and does not include time before that, such as process creation or OS DLL loading.
- `scan_callback` is when the scan completion notification reaches the main thread. `load_call_returned` and `instance_callback` separate the generation API call and completion notification, and `gui_requested` to `app_creation` is the GUI initialization phase.
- `first_audio_callback` is the start of the audio callback; `first_signal_rendered` is after processing the first block that generated samples with an absolute value exceeding 0.000001 (-120 dBFS). This is not the time sound is emitted from speakers.
- The audio thread only obtains time and records it atomically; CSV saving is done on the UI side. Typically, it saves after the first signal generation and catalog preparation are complete. If incomplete, it saves intermediate results after 10 seconds of UI update or on normal exit. Saving may not be possible during UI halt or forced termination.
- Unreached measurement points appear under `# missing:`. For failure details, check the screen and standard error. Refer to [src/startup.rs](src/startup.rs) for measurement point definitions, and [ADR 0034](docs/adr/0034-surge-xt-startup-minimal-data-home.md) for startup acceleration decisions and limitations.

### Recording

When reproducing first-time installation for a demonstration recording, use `cargo build --release --features recording-mode`. On each startup, it creates an empty save destination under `%LOCALAPPDATA%\cat-plugin-player-recordings\`, and does not read/write normal settings/history, detecting only Surge XT / Surge XT Effects CLAP. It does not refer to the cmrt catalog, but acquires a new catalog for Surge. The absence of Surge XT is not faked; the operation assumes a human or coding agent actually uninstalls it before recording. After recording, revert to the normal exe with `cargo build --release`. Refer to [ADR 0026](docs/adr/0026-recording-mode.md) for scope and limitations.

### Surge XT Effects State Table

When Surge XT is updated, etc., run the following in an environment with Surge XT Effects installed to rebuild the table.

```
cargo run --release --example export_surge_fx_states -- ..\cat-music-patterns\surge-xt-fx-states.json
```

Committing/pushing to cat-music-patterns is performed manually. Refer to [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for format and reasons.

※↑AI should write only in this h1 section.

</details>

# Future Brainstorming
- ※Subject to change based on whim
- Extend the strengths not present in [cmrt](https://cat2151.github.io/clap-mml-render-tui/README.ja), such as VST3 and GUI support.
  - Integrate GUI edit results/status with cmrt
    - Load and use edited patches and effects in cmrt
  - Experiment with egui for various things
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, playing multiple SMFs together, swapping patches and effects?
    - Should these be split into separate apps? Or is it better to do them in this app because of patch and effect swapping? What kind of workflow is envisioned?
- Utilize various cmrt crates for different functionalities
  - ※This also refines the crates, and we benefit from using them, so it's a win-win.
  - CLI import / export
    - The cmrt keyboard screen can play patches, effects, arpeggios, etc., from the CLI with a single command. Make the same possible (to the level where changing `cmrt` to `cat-plugin-player` is enough to play).
      - Frequent breaking changes are fine. Manual migration is possible later. Better than being paralyzed by fear of change and halting progress.
  - SMF import / export
    - Currently, output with audio plugin names and patch names recorded in SMF is implemented; this will be further developed.

# Concept, What We Aim For
- Instrument
  - For example, a synthesizer with a MIDI keyboard produces sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations to produce sound.
- Playability
  - We aim to maintain "sound production in the author's environment" as much as possible.
- Instant Playback
  - Perceived 0.2 seconds from app launch to sound. We've optimized for the fastest possible playback.
    - Processes like GUI display preparation are done after the sound.
      - Sound playing before the GUI appears might be considered a "sound splash screen".
- Educational Use
  - It's geared more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - We will frequently introduce breaking changes.

# Random Thoughts Corner
- ※Related to the concept
- I like audio plugin preset patches.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide an instrument UX.
  - Not a DAW UX, but an instrument UX.
- What is an Instrument?
  - It is the sum of each layer from audio plugins to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and playback UI.
        - This is a layer where "vibe coding" can be done.
        - By building this, UX can be improved.
- An instrument for whom?
  - Cats, babies.
    - In other words, broadening the range to include beginners, even targeting cats and babies (e.g., on an Android tablet).
  - ※This is merely a metaphorical expression. Actual operation on Android tablets is unconfirmed. The likelihood of effort being made to make it work on Android if it doesn't is also low.

# Out of Scope, Not Aimed For
- Robustness
  - Absolutely no bugs. The app never crashes, no matter what.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation without CPU load in all environments. Sound never cuts out.
- Requests
  - Immediately responding to all user requests.

# License
- MIT