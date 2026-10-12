# cat-plugin-player

A lightweight app for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and How This App Solves Them
- Audio plugins offer fantastic sounds!
- However, playing them often requires launching a DAW and performing setup tasks.
- Lightweight apps for easy playback sometimes require account registration or have an unintuitive user experience.
- Today, if you want a specific UX, you can achieve it through "vibe coding."
- So, that's what I decided to do.

# Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

# Demo Video

If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as shown. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as described above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking a plugin will load it and automatically start playback.
  - It can play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit patches and effects.
- Patches
  - Each plugin has a variety of preset patches. You can easily select them using its own patch browser (supported by some plugins only).
  - For plugins not supported by the patch browser, you can select patches using the plugin's GUI.
- Effects
  - Each plugin has a variety of preset effects. You can easily select them using its own effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects using the plugin's GUI.

※↑This section is written by a human. AI is prohibited from writing here.

# AI-Managed Section

Will refactor later as it's hard to read. Collapsed for now.

<details><summary>Detailed Features and More</summary>

## Features (Detailed additions by AI)

The display method and constraints for the oscilloscope are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Detection
  - If plugin detection is successful but no instrument is found, an 안내 (guidance) will be displayed to install the free Surge XT via winget. It will also be displayed if only effects are present. If Windows display language is Japanese, it will be in Japanese; otherwise, it will be in English. If `Later` is selected, it will display every time the app starts as long as no instrument is found.
  - `View Terms of Use` opens the Surge XT license (GPLv3) in your browser. Pressing `Agree to Terms and Install` executes winget and shows the stages and elapsed time: "1/4 Preparing," "2/4 Downloading," "3/4 Installing," "4/4 Loading." If the Windows confirmation screen (User Account Control) appears at 3/4, please select "Yes." Upon success, the plugin list will be automatically rescanned, Surge XT (CLAP) will be loaded, and playback will start, similar to `Load`. In this case, previous instrument and effect states are not restored. Even if the prompt is closed, installation continues, and upon completion, a rescan and load will occur.
  - If winget is not available or installation fails, winget's message and manual download links will be displayed and logged to `error.log`. Only Surge XT is included, and the download progress is not shown. The reasons for choosing this are in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- GUI
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, then automatically takes a screenshot. This screenshot is scaled down to a PNG with an aspect ratio maintained, fitting the list display frame and screen magnification. It's saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers (ignoring leading/trailing spaces and case) share the same image for VST3 / CLAP. If name/manufacturer is insufficient, it's distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated and used as shared images from the list/favorite identification. Patch state and favorite load destinations remain distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while maintaining aspect ratio. Hovering over an image shows it at its saved size. Plugins with no GUI displayed show a temporary icon of the same height.
  - The GUI prioritizes placement to the immediate right of the main window, aligned with its top edge, within the screen's working area. If it doesn't fit, it's placed in the bottom-right of the main window's screen working area. Placement occurs on display and GUI resizing, but it doesn't constantly follow main window movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the working area; if its height is greater, its top edge aligns.
  - For blank images or failed captures, retries occur every second for up to 20 seconds from GUI display. Saved blank images are also treated as unsnapped; opening the GUI replaces them with a valid image. No re-capture after success.
  - If capturing fails after retries, an error is displayed. Opening the GUI again retries. GUIs not supporting capture remain with a temporary icon. If you want to retake or if the saved PNG is corrupted, delete the relevant plugin's shared PNG and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are kept during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next launch. Stopped playback pattern selection and confirmed MML/chord text are also restored. If only one instrument candidate exists in the Plugins list on launch, it's automatically loaded if not already, and playback starts with the selected pattern even if the saved state was stopped. If multiple candidates exist, the stopped state is restored. If manually stopped after launch, it remains stopped. Unconfirmed edits are not saved.
  - CLAP Note On/Off is sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of repair and validation records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. Arrows cycle at the ends of the types, and `Stop` is not included. Switching during playback releases all sounding notes of the current phrase and starts from the beginning of the new phrase. When stopped, selecting a type does not play; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except when typing). Playback pattern selection including MML/chords and stop state are restored on next launch. However, when launching with only one instrument candidate, automatic playback occurs from a stopped state.
  - For MML/chord progression audition, open the input screen with the `i` key or `Sequence`'s `Chord` button (hover to see shortcuts and input methods). Example for single notes: `t120 c8r8d4e4r4`; for chord progression: `C F G C`. Both are accepted in the same input field, with chord notation conversion attempted first. Uppercase `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter` / `Confirm` and cancelled with `Esc` / `Cancel`. Cancellation or conversion errors maintain the previous valid phrase. `Space` and `i` within the input screen are for typing; `Space` for play/stop does not work.
  - Confirmed phrases are loop-played with `Play` / `Stop`. Rests, note length, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses normal overall playback. If playback was active just before opening, after confirmation, the new phrase plays from the start; after cancellation, the original playback pattern plays from the start. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to regular playback patterns. Immediately after confirmation, the text and selection/playback state are saved; upon cancellation, the reverted playback state is also saved. This is restored on next launch or from instrument favorites/History. Unconfirmed text is not saved; cancellation or conversion failure preserves the previous confirmed text.
  - Changing input or moving the cursor auditions the single note/chord at that position once. This applies to mid-edit and paste as well. Moving within the same phonetic unit or redrawing alone does not re-sound. Previous tempo, octave, and note length are reflected; it does not sound at rest or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the audition sound ends. The audition sound is released at the next phonetic unit, cancellation, confirmation, or instrument change. Audition does not alter confirmed text or normal selection.
  - For MML loop playback, the Velocity of the sounding note prioritizes the `Velocity` field setting over any in-input specification. CC1 can also be controlled from the `CC1 modulation` field. If the volume of playback events is reached with a very dense phrase, sounding notes are released, and playback stops. It can be restarted with `Stop` → `Play`.
  - When calling from app Favorites/History or changing patches via Browse patches/Browse effects, the stop/play state is maintained. During playback, the phrase starts from the beginning, not continuing any waiting in the middle of the previous phrase. Leading rests specified in MML are preserved. While the patch browser/effect browser is displayed, `Space` for play/stop and MML input start are suppressed. Patch changes made via the plugin's own GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is rebuilt (e.g., on load, state restoration, connection change), to preserve the initial sound, it waits for the first phrase to start while continuing normal audio processing. At 48kHz, the wait is about 21ms for Six Sines and 85ms for TyrellN6. MML's leading rests are added on top of this. For application conditions and real-device verification, refer to [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases values in Note On order within the phrase and decreases them in the next loop, repeating the action. Changes are reflected from the next note without restarting the phrase and are restored on next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or the dropdown. The initial value is 0. `sweep` repeats the action of changing from 0 to 127 over 2 seconds, then 127 to 0 over the next 2 seconds. The selection is restored on next launch.
  - Guitar open strings: MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) are Note On sequentially at 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Off simultaneously, repeating after 0.5 seconds.
  - `Load` success, `Sequence` type selection, start, and stop events are memorized. Memory persists even after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, binary states such as patches and parameters exposed by plugins via UAPMD's state API are saved per plugin to `%LOCALAPPDATA%\cat-plugin-player\states\`. On next launch or reloading the same plugin, these are restored before playback starts.
  - `Load` for an instrument only replaces the sound source, maintaining connected effects and settings. It saves the state of the plugin being replaced, and then switches after successfully generating, restoring, and connecting the new plugin. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. It does not include external sample duplication or saving on abnormal termination.
  - On launch, the previous instrument and effects are directly loaded, states are restored, and then playback begins. After that, the GUI is initialized, and the list is scanned in the background.
  - If direct loading fails due to old history or file movement, it is restored from the scan results after GUI display. Old history is updated once played.
  - If the main instrument is CLAP Floe, when audio processing is rebuilt due to loading or patch restoration, the start of the first phrase is delayed by 100ms. MML's leading rests are added on top of this. This is a temporary measure for observing the symptom of missing initial sounds, and its resolution on real devices is unconfirmed. For application conditions and review policy, refer to [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md).
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - Opening `Browse effects` / `E` / `X` will automatically load a CLAP effect if there is only one candidate, and that slot becomes the target for preset changes. If already connected, it uses that slot without reloading. If no instrument is loaded and there is only one instrument candidate, the instrument is loaded first, and playback begins. If there are multiple instrument candidates, select an instrument first. The number of candidates is determined according to the CLAP priority setting in the Plugins list, and search strings do not reduce the number of candidates.
  - The connection is series: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. `Remove` for a connected effect removes only that one. You cannot connect duplicate effects of the same format + ID.
  - Changing effects via Browse effects requires an instrument. Candidates are obtained from default effect/preset locations explored by the play-server shared core, and no fallback to VST3 occurs. During preparation, current playback continues, and loading, scanning, and patch operations are disabled. While stopped, it remains stopped; during playback, the current phrase restarts from the beginning, maintaining MML/Velocity/CC1 global selection and other effects' order/settings/Bypass. On generation/application/connection failure, it reverts to the original configuration. After success, the applied state and connection are saved for the next launch, and Favorites/History are not added. Save failure is displayed as already applied.
  - The `Browse effects` next to `Sequence` or the `E` / `X` keys open a maximized, independent effect browser window, allowing you to select presets from a list. It cannot be opened while editing MML. Filter by Category→Kind on the left and by search. Classification and selection target are the same as cmrt's effect selector. Search is case-insensitive regular expression, space-AND, `-condition` for exclusion, applied to preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regex during input displays an error and preserves the previous list. The top row has Search, Target slot, and `Random`. The search field uses the remaining width, and the `/` key in the browser moves input to the search field. `R` randomly selects from filtered results. Multiple column display, moving with arrow keys / `h` / `j` / `k` / `l` / numeric repetition / `PageUp` / `PageDown`, and closing with `Enter` / `Esc` are the same as Browse patches. In results with mixed effects, effect names are given per effect, and if Kind differs, preset names are color-coded per Kind.
  - The target slot is chosen from `Add new effect` or `Replace n` of a connected effect. When opened, it defaults to the slot last written by this browser, otherwise the last effect, or `Add new effect` if no effects are present. Applying with `Add new effect` makes the added effect the target, and subsequent selections replace that slot, so the number of effects doesn't increase by continuing to select. If the target slot is removed, the last effect becomes the target. Presets for connected effects in non-target slots are grayed out; selecting them displays a reason and does not change. Presets entered into the target slot via this browser are marked with `▶`, and opening the browser selects them (does not re-apply).
  - When another candidate is selected by click/key movement or filtering, an immediate application request is made. Simply opening does not change the effect. You can select the next during preparation, uninitiated selections are replaced by the latest one, and closing cancels them. Category, Kind, search string, list selection position, and target slot are saved on exit and restored on next launch. Key movement starts from the restored selection. Category/Kind are restored by name; if gone, they revert to `ALL` in that field. Selection is restored by preset; if gone from the catalog, it becomes unselected, and key movement starts from the beginning. The target slot is restored by its position in the chain; if no effect exists at that position, it becomes the default slot when opened.
  - If the effect before or after replacement in Browse effects is CLAP Surge XT Effects, the sound just before switching fades out over approximately 50ms, and the sound after switching fades in over approximately 50ms. This is a temporary measure to suppress loud "pops" caused by residual sound from the instrument passing through the new effect (especially Distortion types). The switch is delayed by that amount (max approx. 70ms), and residual sound may be faintly audible during fade-in. Fading does not occur for switches between other effects or for patch/instrument changes. Refer to [ADR 0027](docs/adr/0027-surge-xt-effect-swap-fade.md) for reasons and scope.
  - For CLAP Surge XT Effects presets, instead of temporarily launching the plugin for preparation, a pre-prepared state table is fetched on each launch from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) (saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-fx-states.json`). This avoids a bug where the app crashes if you keep selecting presets. If it cannot be fetched, it uses the previously saved table; if that also doesn't exist, Surge XT Effects presets won't appear as candidates in the effect browser, and a reason will be displayed. The table is created from Surge XT 1.3.4 presets; presets with different content (e.g., different Surge XT versions) are excluded from candidates, and the count of excluded items is displayed. Refer to [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for reasons.
  - `Routing` displays the connection order. You can change the order by dragging the `↕` button of an effect up or down. While dragging, the moving effect's name and insertion line are shown. Instruments are not subject to reordering. You can edit patches and effects via `Show UI` for each plugin. `Instrument` / `Effect` are displayed with the same type color as the left list.
  - Removing all effects reverts to a direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing audio to pass through while retaining the instance and settings. It can also be toggled with the `b` key (does not work in MML input screen, Browse patches/Browse effects display, or while typing). While bypassed, connected effects in Routing are grayed out, with `Bypassed` appended.
  - `Sequence` is for instruments. Stopping the sequence continues audio processing to allow effect tails to decay.
  - On next launch, instrument, all effect orders, their respective states, and Bypass status are restored before playback. History of 1 old-format effect can also be loaded. If effect restoration fails, an error is displayed, and it reverts to a direct instrument connection; the saved configuration is retained.
  - Add, remove, and reorder operations are saved only if the new connection successfully prepares. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication; stereo → mono is the average of left/right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause a short audio dropout, and sounding notes and tails may be cut off. Side-chaining, parallel connections, and host-side Dry/Wet are not supported.
- Patches
  - If no instrument is loaded and there's only one instrument candidate, `Browse patches` / `T` automatically loads it and starts playback. Requests during loading/scanning are processed after completion, and closing the list cancels uninitiated auto-loading. If multiple candidates exist, it does not auto-select. Candidate count determination follows the CLAP priority setting in the Plugins list, and search strings do not reduce the candidate count. Load failures are displayed, and no automatic retry occurs. The supported formats and catalog limitations for the patch list itself remain as before.
  - `Browse patches` next to `Sequence` or the `T` key opens an independent maximized patch browser window on the screen where the main window resides. The main window's position/size is not changed. It cannot be opened while editing MML. Filter by Role→Preset→Search within the framed area. Roles/Presets with no patches are not displayed. Items that become 0 after Solo/Mute filtering (e.g., `FM Bass` when everything except Dexed is soloed) are also hidden. They are not hidden by search string, and custom conditions are displayed even if 0 items, to allow deletion. The left condition area and right Patches list are individually scrollable. They use the remaining width and height, operable even when unmaximized or in narrow windows. The top row has Search, `Solo/Mute`, `Random`, and `☰`. The search field uses the remaining width, and the `/` key in the browser moves input to the search field. Instrument condition details are available on `Solo/Mute` hover. Clicking this button or the `M` key in the browser opens a vertically arranged Solo/Mute overlay; `OK` or `Enter` confirms all at once. Solo/Mute are displayed immediately to the right of the instrument name. Re-clicking the selected mode reverts to normal; `Esc` / clicking outside the overlay discards changes. Multiple Solos are treated as OR, Mute as exclusion. Search uses case-insensitive regular expressions, space-AND, `-condition` for exclusion, and `plugin:` condition (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases for integrated patches are also searchable. Invalid regex results in an error, stopping application,抽選 (random selection), and condition saving. If 0 items, review conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 items. Role, Preset, and search string are saved on exit and restored on next launch. Preset is restored by name; if the condition is deleted, it reverts to `ALL` for that Role. If the selected Role becomes 0 items on restoration or Solo/Mute change, it reverts to `ALL` overall; if Preset becomes 0 items, it reverts to `ALL` for that Role.
  - The list displays patch name cells on a single, non-wrapping line, arranged in multiple columns according to width, from left → right → next line. In narrow windows, it reverts to 1 column. Resizing maintains the selected patch, and changing only the number of columns does not reload. Clicking a patch does not move the list's display position. Up/down/left/right keys or `h` / `j` / `k` / `l` move according to the visual arrangement. Left/right move to previous/next patch (at row end, to the end of the adjacent row); up/down move to the row above/below; to a short last row with no patch directly below, it moves to the end of that row. Typing a number first, like `2j` or `10k`, moves that many times (stops at ends; other keys cancel the number). `PageUp` / `PageDown` move 10 times up/down; in combination with a number, it moves that multiple. Key movement, Random, filtering, and column count changes only scroll to make the selected row visible if it's off-screen. If all patches fit, it does not scroll. For single-instrument results, the instrument name is omitted; for mixed results, it's retained. Text color indicates only one difference within the results. If instruments are mixed, the instrument name part is color-coded per instrument. For a single instrument, if Preset is `ALL`, patch names are color-coded per relevant Preset; if that doesn't distinguish, then per category. Colors are Monokai-like. If all are the same, normal text color is used. Colors cycle through 5 hues, so distant separate groups may have the same color. For pink patches, pink patch names are prioritized. Classification folders etc. are omitted; if distinguishing same-name patches, necessary paths are retained. Dexed program specification is maintained. Hovering shows the full display path, source path, valid category, and merge count if 2 or more. Load/Size are not permanently shown in the list; `no category` / `1 merged` are also not shown. Normal patches request immediate application when selection changes by mouse/key movement, and the same action occurs when a different candidate is selected by condition change. Simply opening does not change the patch. When opened, if the currently playing patch (applied via Browse patches) is in the current results, that patch is selected and scrolled into view (not re-applied). The playing patch is saved on exit and inherited as the restored patch on next launch. It is retained through effect addition/deletion/Browse effects. If the playing patch is not in the results or is unknown, the selection from when last closed is maintained (unselected when opening for the first time after launch). After changing patches via Favorites/History or instrument load/delete, the playing patch is unknown, and selection is cleared. Key movement starts from the selection when opened; if unselected, it moves to the top. During preparation, the next selection can be made, uninitiated requests are replaced by the latest one. On successful normal application, duplicate patch names or fixed padding at the bottom are not displayed. Only if selected and applied patches differ, a brief guide is shown, indicating that the previous patch might continue to play for unconfirmed pinks, etc. Pink Space guide and application failure/save errors are displayed.
  - Pink patches with sample size 64,000,000 bytes or more are not automatically applied (unknown size treated as normal). After selection, `Space` opens a confirmation overlay for instrument, patch, and size; pressing `Enter` within it starts preparation/application. `Esc` during confirmation cancels without changes. Even once applied, a patch requires re-confirmation. A progress overlay is shown during loading, and it cannot be cancelled mid-way. `Space` / `Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is same as `OK`).
  - `Random` or the `R` key within the browser selects uniformly from the displayed results. While typing, shortcuts `T` / `R` / `M` / `/` / `h` / `j` / `k` / `l` / numbers / `Enter` do not work, and `R` / `M` do not work while overlay / save menu is displayed in the browser. For pink, selection only, the same Space→Enter confirmation is required. `Esc` / `Enter` or the OS close of the browser window closes only the browser, preserving the main app and the last successfully applied patch, discarding uninitiated requests and old preparation results. Already started processing is completed. `Esc` / clicking outside the overlay when confirmation overlay / Solo/Mute / save menu is open closes only the child, not passing the same input to the browser or background operations. OS close closes the entire browser. It can be reopened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to Role via `Save condition to Role` in the hamburger `☰` menu and deleted via `Delete` in Preset. The addition target from ALL is Etc. They are handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` in the `cat` config directory. On corrupted load, the original file is protected, writing is stopped, and save failures are also displayed on screen. Role / Preset / search / Solo/Mute are retained during the same app launch, but only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared or migrated.
  - During Browse patches preparation, current playback continues, and conflicting load/scan/patch operations are disabled. There may be waits for sample loading by Floe etc. MML/Velocity/CC1 global selection, effect order/state/Bypass are maintained, and after success, the next launch state is saved. Favorites/History are not automatically added. Application success and save failure are displayed separately. Progress of preparation/application is updated even when child windows are focused.
  - Patch browser candidates are CLAP patches from Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, and Vaporizer2 in the existing cmrt catalog, where the format + ID matches an installed instrument.
  - The patch browser catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode. VST3 fallback and unknown instruments are not covered. Effect preset selection uses Browse effects in the effect routing section. Dexed retains program specification within cartridges. Floe/sforzando external samples are required at their original locations; there is no function to duplicate or save samples.
  - In environments without cmrt and where the above `catalog.json` does not exist, Surge XT's patch list is fetched on each launch from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns), saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json`, and used as Browse patches candidates. If it cannot be fetched, the previously saved list is used; if that also doesn't exist, a load failure is displayed in the patch browser. A "loading" display will appear until fetching is complete (up to 10 seconds). The list is generated from Surge XT 1.3.4; for different Surge XT versions, added patches will not appear, and selecting a non-existent patch will display a preparation failure. If `catalog.json` exists but cannot be read, it does not switch. Reasons are in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - If preparation/application fails, the reason is displayed. The original configuration, state, and playback state are restored, and instances where state restoration also failed will suppress saving. Save failures after patch application are displayed as already applied. Instrument-specific preparation/reflection conditions and validation scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - You can save multiple `★ Add favorite` instances of the current patch and settings as favorites for each plugin. For instruments, it also saves the playback pattern, stopped selection, and confirmed MML/chord text. For effects, it does not save playback settings. Names are automatically assigned and can be changed later.
  - Loading a different plugin automatically records the patch, settings, and playback pattern of the replaced plugin to History before switching. The name is `PluginName SequenceNumber`. It is not added for reloading the same plugin or restoring on launch. Confirmed MML/chord text and stopped selection for instruments are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is aborted.
  - During manual/auto save, it checks for duplicates of the same plugin, role, playback pattern, confirmed MML text, and state, and reuses existing items and names. For History's auto-record, only stop differences are treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - The [Plugin-specific processing policy/list (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides exceptions, reasons, scope, and limitations for each product. For confirmed TyrellN6 CLAP state format, the compressed part that changes only with playback is excluded from duplicate judgment, and patches with the same text-based patch settings are grouped. Confirmed PCore `UI_op=9/10` and trailing empty lines of text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed Vaporizer2 CLAP format, minute differences in MSEG time/coordinates (absolute difference of specified items: time ≤ `0.00011` ms, normalized coordinates ≤ `0.000001`. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) that arise during loading/saving are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - Switch lists using the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order, with new additions placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON allows reordering with `↑` / `↓` at the end of the row, saving immediately. Row click loading and effect removal work as before. `Sort by plugin name` sorts once by plugin name in ascending order and saves (case-insensitive, preserving relative order for same names). Manual adjustments are possible afterwards. Reordering is disabled while filtering, and reordering mode is OFF on launch. See [ADR 0017](docs/adr/0017-manual-favorites-order.md) for adoption background and specifications.
  - History is sorted by the latest registration order. If determined to be the same patch, the saved patch is reused, its registration time is updated, and it is moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are maintained. All old-format items are migrated to History, and all items not starting with `auto ` are also kept in Favorites (because old formats do not record the origin of automatic saves, this ensures all renamed automatic saves are included in History). Refer to [ADR 0016](docs/adr/0016-history-instead-of-automatic-favorites.md) for policy and reasons.
  - History does not display Rename/Delete operation menus. Renaming/deletion is done in Favorites.
  - The `Show only CLAP for duplicate plugins` setting in the global `☰` is ON by default. If CLAP plugins with the same name, manufacturer, and type exist, VST3s in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are displayed). Turning OFF shows both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and persists on next launch. Favorites and the format loaded/restored are not changed.
  - For both tabs, clicking the name/image loads; for connected effects, clicking again removes. The currently loaded/connected row is highlighted, and the color persists even when effects are bypassed. The `Load` / `Add` / `Remove` buttons in Plugins perform the same operations.
  - Favorites are displayed one item per row. Long names/supplementary info are truncated; hover to see full text. Waveform/spectrum are shown below Routing on the right; the right panel width can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display to bottom-right / bottom of screen (default is bottom-right on launch).
  - For sforzando CLAP's confirmed state format, it reuses existing favorites without increasing identical patches based only on save counter and note/CC1 playback value differences. Patch setting differences are preserved. Refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` is displayed to the left of each row. Clicking a favorite name restores the patch/settings. For instrument favorites, it restores the saved confirmed text and playback pattern selection. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites in the old format without text retain the current text, and old-format stopped items maintain the current selection. If conversion or state load/connection fails, the original text/selection/playback state is restored, and the failed content is not saved to the session. Instrument favorites maintain connected effects; effect favorites maintain the instrument and MML. Calling an effect requires an instrument.
  - Connected effect favorites display `Connected - click again to remove`, and re-clicking the name removes that effect. This applies even if settings are edited after calling or if bypassed. The instrument, other effects, and saved favorites remain. Another patch of the same format + ID switches settings at the current position, and unconnected effects are added to the end. Selected favorites for each effect are also restored on next launch.
  - Rename/Delete operations are available from the `…` at the end of the row. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Short audio dropouts may occur during save/restore, and the scope of what can be saved is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right-hand analysis panel. If `☰` → `On right` for the oscilloscope is unchecked, spectrum, oscilloscope, and Lissajous are arranged horizontally at the bottom of the screen.
  - The Lissajous display shows the last 50ms of output (max 4096 samples) after effects and Bypass, rotated 45°. A common auto-scale maintains volume differences and aspect ratio; in-phase at the same level results in a vertical line, anti-phase a horizontal line, and sine waves with phase differences become ellipses or circles. Output only on the left is a diagonal from top-left to bottom-right, only on the right from top-right to bottom-left; mono is a vertical line; silence is a central dot.
  - It updates independently of notes, cycle counts, or trigger settings, and `Show analysis labels` displays headings. Axis and rendering methods can also be checked in the graph tooltip.
  - `Correlation` directly below Lissajous displays the correlation between left and right channels for the last 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is anti-phase. Negative values are shown in red, serving as a guide for cancellation when mixed to mono. It shows "—" for silence or if one side is silent.
- Waveform Display
  - `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) channels of the output (reflecting effects and Bypass) overlaid. Mono displays the same waveform on both.
  - The display width is determined from the MIDI note number of the last Note On (A4 = 440Hz), and `1`, `2`, `4`, `8 cycles` can be selected. After Note Off, the tail is displayed based on the last note. Output containing chords or detune may not repeat perfectly at that width.
  - `Zero cross` aligns to the negative→positive crossing of the left channel. If the left channel is silent, it uses the right channel; if no crossing, it displays from the beginning.
  - `Similarity` exhaustively checks the position (one sample at a time, for the display width from the buffer start) where the correlation coefficient with the previous display is maximized. For ties, the earlier position is adopted, and the same start position is used for both left and right. For the first time or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when a note is sounded, playback pattern changes, display cycle count, trigger method, or audio stream changes. A waiting display appears until sufficient data is gathered.
  - Exploration is performed on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - Initial display settings are 4 cycles and zero cross. Display settings are not saved.

## Settings: Window Position and Size

You can specify the main window's launch position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the application. Session and favorite selection states are saved to `status.json` in the same directory, and regular saves do not overwrite TOML. If a mixed state exists in old TOML, it's migrated on first launch, saving the original file as `config.toml.before-status-migration.bak` before removing only the state items. Comments, formatting, and unknown items are preserved, and independent comments attached to deleted items are moved to the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The `x` and `y` for the main window are logical coordinates and are used as the launch position if both are specified. If omitted, it defaults to the traditional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. Either can be specified alone; if omitted or less than or equal to 0, or non-finite, they revert to width 900, height 600. Changes are reflected on next launch. The plugin placement method is currently `right_then_bottom_right` only, and it behaves the same when omitted. GUI size and placement are calculated based on actual window frame, screen working area, and DPI. Automatic saving of manually moved positions for the main window or GUI, or per-plugin position specification, are not implemented.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then run from this app's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` are always referenced locally from Cargo.toml. Play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to the traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` needs to re-export them. The patch browser uses the public Role/Preset/filter API and tui-core's sample size determination. Browse effects uses play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions, and tui-core's search conditions. Insufficient checkout or API will result in build errors; no automatic switch to Git versions.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The location of UAPMD at build time is specified by an environment variable. `config.toml` is not read as build settings.

Do not use `cargo install`; directly run the exe generated in the default `target/release`. Building uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require reconstruction and rebuilding. If `target/shim` was created with a different version of VS (e.g., VS 2022), it will continue to build with that version, so delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe and expanded to the cache directory for loading when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency configuration and operational policy. Local TUI switching is not needed for building `cat`. For cross-repository building with TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; `off` is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. When verifying, record the HEAD and diff of each repo.

Launches the GUI without arguments. `--help` and `--version` are also available.

## For Developers

### Recording

When recreating first-time setup for an introductory recording, use `cargo build --release --features recording-mode`.
It creates an empty save destination under `%LOCALAPPDATA%\cat-plugin-player-recordings\` on each launch.
It does not read/write normal settings/history and only detects Surge XT / Surge XT Effects CLAP.
It does not refer to the cmrt catalog and newly acquires the Surge catalog.
Surge XT's absence is not spoofed; operation is to actually uninstall it manually or via a coding agent before recording.
After recording, return to the normal exe with `cargo build --release`.
Refer to [ADR 0026](docs/adr/0026-recording-mode.md) for scope and limitations.

### Surge XT Effects state table

When updating Surge XT, etc., run the following in an environment with Surge XT Effects installed to regenerate the table:

```
cargo run --release --example export_surge_fx_states -- ..\cat-music-patterns\surge-xt-fx-states.json
```

Committing and pushing to cat-music-patterns is done manually. Refer to [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for format and reasons.

※↑AI, please only write in this h1 section.

</details>

# Future Brainstorming
- ※Subject to change based on mood.
- Extend the strengths not found in cmrt, such as VST3 and GUI support.
  - Coordinate GUI edit results with cmrt.
    - Utilize edited patches and effects read by cmrt.
  - Experiment with egui.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, playing multiple SMFs together, swapping patches and effects?
    - Should these be separated into different apps? Or is it better to do it in this app because of patch and effect swapping? What kind of workflow is envisioned?
- Utilize various cmrt crates.
  - ※This will also refine the crates, and we benefit from using them, so it's a win-win.
  - CLI import / export
    - The cmrt keyboard screen can play patches, effects, and arpeggios with a single command from the CLI. Make the same possible here (to the level where it works by simply changing `cmrt` to `cat-plugin-player`).
      - Frequent breaking changes are fine. Manual migration is possible later. It's better than being paralyzed by fear of change and halting progress.
  - SMF import / export
    - Currently, recording audio plugin names and patch names into SMF and outputting them is implemented; develop this further.

# Concept, Goals
- Musical Instrument
  - For example, a synthesizer with a MIDI keyboard makes sound when you turn it on and press a key.
    - Our goal is to make it as easy to use as that.
      - Minimize operations to produce sound.
- Sound Production
  - Aim to maintain "it sounds on the author's environment" as much as possible.
- Immediate Sound
  - Feel the sound within 0.2 seconds from app launch. Playback is prioritized for maximum speed.
    - Processes like GUI display preparation are done after the sound.
      - Sound playing before GUI display might be considered the sound version of a splash screen.
    - You need to run the release-built exe directly (because `cargo` introduces timestamp checks, and debug builds have debug code, both causing launch delays).
- Educational Use
  - It leans more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

# Miscellaneous Thoughts Section
- ※Related to the concept.
- I like audio plugin preset patches.
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide a musical instrument UX.
  - I want to provide a musical instrument UX, not a DAW UX.
- What is a musical instrument?
  - It is the synthesis of layers from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and playback UI.
        - This is a layer where "vibe coding" can be applied.
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - That means targeting beginners, including cats and babies (e.g., Android tablets).
  - ※This is merely a metaphor. It is unconfirmed whether it actually works on Android tablets. The likelihood of efforts to make it work on Android if it doesn't is also low.

# Out of Scope, Not Goals
- Robustness
  - Absolutely no bugs. The app never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every possible feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed without CPU load in all environments. Audio never cuts out.
- Demands
  - Immediate response to all user requests.

# License
- MIT