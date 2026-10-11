# cat-plugin-player

A lightweight app for easily playing audio plugins. It uses [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Current Challenges and What This App Solves
- Audio plugin sounds are amazing!
- Often, to play them, you need to launch a DAW and perform preparatory steps.
- Lightweight apps for casual playing sometimes require account registration or have a UI/UX that isn't for me.
- Nowadays, if you have a desired UX, you can achieve it through vibe coding.
- So, I decided to do just that.

# Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

# Demo Video: Automatic Audio Plugin Installation, Browsing Patches & Effects

If you don't have audio plugins installed, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) for you, like this. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If you don't have audio plugins installed, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as described above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking a plugin will automatically start playback after the plugin loads.
  - It can play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit patches and effects.
- Patches
  - Each plugin has a variety of preset patches. You can easily select them using the app's unique patch browser (supported by some plugins only).
  - For plugins not supported by the patch browser, you can select patches using the plugin's GUI as usual.
- Effects
  - Each plugin has a variety of preset effects. You can easily select them using the app's unique effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects using the plugin's GUI as usual.

※The above section is written by a human. AI is prohibited from writing here.

# Features (Detailed additions by AI. Planning to revise later as it's hard to read.)

Oscilloscope display methods and constraints are documented in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Detection
  - If plugin detection is successful but no instrument is found, a prompt to install the free Surge XT via winget will be displayed. It also appears if only effects are present. The prompt will be in Japanese if Windows display language is Japanese, otherwise in English. Even if you choose `Later`, it will reappear every time the app launches as long as no instrument is present.
  - Clicking `View Terms` opens the Surge XT Terms of Use (GPLv3) in your browser. Pressing `Agree to Terms and Install` executes winget, displaying stages and elapsed time: '1/4 Preparing', '2/4 Downloading', '3/4 Installing', '4/4 Loading'. If a Windows confirmation screen (User Account Control) appears at 3/4, select 'Yes'. Upon success, the plugin list will be automatically rescanned, Surge XT (CLAP) loaded, and playback will start as with `Load`. Previous instrument/effect restoration is not performed at this time. Even if the prompt is closed, installation continues, followed by rescanning and loading upon completion.
  - If winget is not present or installation fails, the winget message and manual download link are displayed and logged to `error.log`. Only Surge XT is included, and no download percentage is shown. The reason for this choice is documented in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- GUI
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, then automatically takes a screenshot, scales it down to fit the list display frame and screen zoom level while preserving aspect ratio, and saves it as a PNG to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`. Plugins with matching name + manufacturer will share the same image for both VST3 / CLAP (ignoring leading/trailing spaces and case). If name/manufacturer is missing, they are distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated and used as shared images based on list and favorite identification information. Patch states and favorite load destinations are still distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to button height while preserving aspect ratio. Hovering over an image displays it at its saved size. Plugins without a displayed GUI show a placeholder icon of the same height.
  - The GUI prioritizes placement to fit within the screen's working area, to the right of and aligned with the top of the main window. If it doesn't fit, it's placed in the bottom-right of the working area of the screen where the main window is located. It is positioned on display and when the GUI is resized, but does not constantly follow main window movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the working area's left edge; if its height is greater, its top edge aligns with the working area's top edge.
  - In case of blank images or failed captures, retries are performed at 1-second intervals for up to 20 seconds from GUI display. Saved blank images are treated as uncaptured and will be replaced with valid images when the GUI is opened. No re-capture after successful capture.
  - If capture fails after retries, an error is displayed. Reopening the GUI retries the capture. GUIs that don't support capture remain with a placeholder icon. If you wish to recapture or if the saved PNG is corrupted, delete the shared PNG for the relevant plugin and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are retained during migration).
- Playback
  - The last played instrument, connected effects, Bypass status, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next launch. Stopped playback pattern selections and confirmed MML/chord text are also restored. If only one instrument candidate is in the Plugins list at startup, it is automatically loaded if not already loaded, and playback starts with the selected pattern even if a stopped state was saved. If multiple candidates exist, the stopped state is restored. If manually stopped after startup, the stopped state is maintained. Unconfirmed edits are not saved.
  - CLAP Note On/Off messages are sent according to the input format supported by the plugin. This also applies to sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or a dropdown. Arrows cycle through the ends of the types; stop is not included. Switching during playback releases all notes of the current phrase and starts from the beginning of the new phrase. When stopped, selecting a type does not play; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except when typing text). Playback pattern selections including MML/chords and stop state are restored on next launch. However, if only one instrument candidate is present at startup, it auto-plays from the stopped state.
  - To preview MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). An example of single notes is `t120 c8r8d4e4r4`, and a chord progression example is `C F G C`. Both are accepted in the same input field, and chord notation conversion is attempted first. Capital `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter`/`Confirm` and canceled with `Esc`/`Cancel`. Cancellation or conversion errors preserve the previous valid phrase. `Space` and `i` within the input screen are used for text entry, and `Space` for play/stop does not work.
  - Confirmed phrases are looped with `Play`/`Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses the overall normal playback. If playback was active just before opening, after confirmation the new phrase plays, and after cancellation the original playback pattern restarts from the beginning. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to regular playback patterns. Immediately after confirmation, the text and selection/playback state are saved; upon cancellation, the reverted playback state is saved. These are restored on next launch or from instrument favorites/History. Unconfirmed text is not saved, and cancellation or conversion failure preserves the previous confirmed text.
  - Changing input or moving the cursor previews the single note/chord at that position once. This also applies to mid-edit changes and pasting. Moving within the same phonetic unit or redrawing alone does not re-trigger playback. It reflects the previous tempo, octave, and note length, and does not sound at the position of rests or commands. Normal playback stops when editing is opened, and overall playback does not resume after the preview sound ends. The preview sound is released upon the next phonetic unit, cancellation, confirmation, or instrument change. Previews do not alter confirmed text or regular selection.
  - The velocity for confirmed MML loop playback prioritizes the `Velocity` field setting over any in-input specification. CC1 can also be controlled from the `CC1 modulation` field. If the performance event capacity is reached due to a very dense phrase, active notes are released, and playback stops. You can restart with `Stop` → `Play`.
  - When changing patches via Favorites/History or Browse patches/Browse effects, the stop/play state is maintained. During playback, it starts from the beginning of the phrase and does not inherit any mid-phrase waits from before the change. Leading rests specified in MML are preserved. While the patch browser or effect browser is displayed, `Space` key play/stop and MML input initiation in the background are suppressed. Patch changes within a plugin's native GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is rebuilt during load, state restoration, or connection changes, it waits for the first phrase to start while continuing normal audio processing to preserve the initial sound. The wait time at 48kHz is approximately 21ms for Six Sines and 85ms for TyrellN6. Leading rests in MML are added to this. See [ADR 0019](docs/adr/0019-random-patch-all-catalog.md) for application conditions and actual device verification.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, or `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases the value in Note On order within the phrase and decreases it in the next loop, repeating this behavior. Changes are reflected from the next note without restarting the phrase and are restored on next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, or `sweep` using left/right arrows or a dropdown. The initial value is 0. `sweep` cycles by changing from 0 to 127 over 2 seconds, then 127 to 0 over the next 2 seconds. The selection is restored on next launch.
  - Guitar open strings (E-A-D-G-B-E, MIDI note numbers 40, 45, 50, 55, 59, 64) are triggered with Note On sequentially at 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Offed simultaneously, and this repeats after 0.5 seconds.
  - Upon successful `Load`, it remembers the `Sequence` type selection, start, and stop times. Memory persists after stop or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - Upon exit and `Remove`, the binary state (patches, parameters, etc.) exposed by the plugin via UAPMD's state API is saved per plugin to `%LOCALAPPDATA%\cat-plugin-player\states\`. This is restored before playback starts on next launch or when the same plugin is reloaded.
  - An instrument `Load` replaces only the sound source, maintaining connected effects and their settings. The state of the plugin being replaced is saved, and switching occurs only after successful generation, restoration, and connection of the new plugin. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. Duplication of external samples and saving during forced termination are not included.
  - On startup, the previous instrument and effect are loaded directly, and their state is restored before playback begins. The GUI is then initialized, and the list is scanned in the background.
  - If direct loading is not possible due to old history or file movement, it is restored from the scan results after GUI display. Old history is updated once played.
  - If the main instrument is CLAP Floe, when audio processing is rebuilt (e.g., during loading or patch restoration), the start of the first phrase is delayed by 100ms. Leading rests in MML are added to this. This is a temporary measure for observing symptoms where initial notes disappear, and actual device resolution is unconfirmed. See [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - For startup time measurement methods and results, please refer to [docs/startup-timing.md](docs/startup-timing.md).
- Effect Routing
  - Opening `Browse effects`/`E`/`X` automatically loads a compatible CLAP effect if only one candidate is available, making that slot the target for preset changes. If already connected, it uses that slot without reloading. If no instrument is loaded and only one instrument candidate exists, it loads the instrument first and starts playback. If multiple instrument candidates exist, please select an instrument first. The number of candidates is determined according to the CLAP priority setting in the Plugins list; search strings do not reduce the number of candidates.
  - This is a serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects to the end. Connected effects can be removed one by one with `Remove`. Duplicate effects of the same format + ID cannot be connected.
  - An instrument is required for effect changes via Browse effects. Candidates are retrieved from the default effect/preset locations scanned by the play-server shared core; VST3 alternatives are not used. During preparation, current playback continues, and loading, scanning, and patch operations are disabled. While stopped, it remains stopped; while playing, the current phrase restarts from the beginning, and global MML/Velocity/CC1 selections and other effect order/settings/Bypass states are maintained. On failure to generate, apply, or connect, it reverts to the original configuration. Upon success, the applied state and connection are saved for next launch, and Favorites/History are not updated. Save failures are displayed as applied.
  - Using `Browse effects` next to `Sequence` or the `E`/`X` keys opens a separate maximized effect browser window, allowing you to select presets from a list. It cannot be opened while MML is being edited. Filter by Category → Kind on the left and then by search. Classification and selection targets are the same as cmrt's effect selector. Search uses case-insensitive regular expressions, space for AND, and `-condition` for exclusion, applying to preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regular expressions entered mid-way display an error and retain the previous list. The top row shows Search, Target slot, and `Random`. `/` within the browser moves input to the search field, and `R` randomly selects from filtered results. Multi-column display, navigation with arrow keys/`h`/`j`/`k`/`l` (with numeric repetition), `PageUp`/`PageDown`, and `Enter`/`Esc` to close are the same as Browse patches. In results with multiple effects, effect names are appended per effect, and preset names are color-coded per Kind if Kinds differ.
  - The target slot is chosen from `Add new effect` or `Replace n` of a connected effect. When opened, it defaults to the slot last written to by this browser, otherwise the last effect, or `Add new effect` if no effects are present. Applying `Add new effect` makes the added effect the target, and subsequent selections replace that slot, so the number of effects does not increase by continuously selecting. If the target slot is removed, the last effect becomes the target. Presets for effects connected to non-target slots are displayed in gray; selecting them shows a reason and does not change them. Presets applied to the target slot via this browser are marked with `▶`, and opening makes that preset selected (no re-application).
  - If the effect being replaced or the replacement effect in Browse effects is CLAP Surge XT Effects, the sound just before switching fades out over approx. 50ms, and the sound after switching fades in over approx. 50ms. This is a temporary measure to prevent a loud 'pop' sound caused by leftover instrument resonance passing through the new effect (especially Distortion types). The switch is delayed by that amount (max approx. 70ms), and leftover resonance may be faintly audible during the fade-in. No fading occurs for switches between other effects or for patch/instrument changes. See [ADR 0027](docs/adr/0027-surge-xt-effect-swap-fade.md) for reasons and scope.
  - For CLAP Surge XT Effects presets, instead of temporarily launching the plugin for preparation, a table of pre-prepared states is fetched on each launch from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) (saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-fx-states.json`). This avoids a bug where the app crashes if presets are continuously selected. If fetching fails, the previously saved table is used; if that's also unavailable, Surge XT Effects presets won't appear as candidates in the effect browser, and a reason will be displayed. The table is generated from Surge XT 1.3.4 presets, and presets with different content (e.g., from different Surge XT versions) are excluded from candidates, with the count of excluded items displayed. See [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for reasons.
  - The `Routing` displays the connection order. Effects can be reordered by dragging their `↕` buttons up or down. During drag, the moving effect name and insertion line are shown. Instruments are not subject to reordering. You can edit patches and effects using `Show UI` for each plugin. `Instrument`/`Effect` are displayed with the same type-specific color as the list on the left.
  - Removing all effects reverts to a direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing signals to pass through while retaining instance and settings. It can also be toggled with the `b` key (does not work in MML input screen, while Browse patches/Browse effects are displayed, or when typing text). While bypassed, connected effects in Routing are shown in gray, with `Bypassed` appended.
  - `Sequence` is for instruments. Even if `Sequence` is stopped, audio processing continues, allowing effect tails to decay.
  - On next launch, it restores the instrument, the order of all effects, their respective states, and Bypass status before playback. History for a single effect in the old format can also be loaded. If effect restoration fails, an error is displayed, and it reverts to a direct instrument connection, while keeping the saved configuration.
  - Addition, removal, and reordering are saved only if the new connection is successfully prepared. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication, stereo → mono is the average of left/right. Auxiliary input is silent.
  - Connection and Bypass changes may cause a brief audio glitch, and notes currently playing and their tails may be interrupted. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Patches
  - If no instrument is loaded and only one instrument candidate exists, `Browse patches`/`T` automatically loads it and starts playback. Requests during loading/scanning are processed after completion, and closing the list cancels any pending auto-load. If multiple candidates exist, it does not auto-select. The number of candidates is determined according to the CLAP priority setting in the Plugins list; search strings do not reduce the number of candidates. Load failures are displayed, and no automatic retries are performed. The supported formats and catalog limitations for the patch list itself remain as before.
  - Using `Browse patches` next to `Sequence` or the `T` key opens a separate maximized patch browser window on the same screen as the main app. It does not change the main app's position or size. It cannot be opened while MML is being edited. Filter by framed Role → Preset → Search. Roles/Presets with no patches are not displayed. Items that result in 0 entries after Solo/Mute filtering (e.g., `FM Bass` when everything but Dexed is soloed) are also hidden. Search string does not hide items, and custom conditions are displayed even if 0 entries so they can be deleted. The condition area on the left and the Patches list on the right are independently scrollable. The list uses the remaining width and height, operable even when unmaximized or in a narrow window. The top row shows Search, `Solo/Mute`, `Random`, and `☰`. The search field uses the remaining width, and the `/` key within the browser moves input to the search field. Detailed instrument conditions can be checked by hovering over `Solo/Mute`. Clicking this entire button or the `M` key within the browser opens a vertically stacked Solo/Mute overlay, which can be confirmed collectively with `OK` or `Enter`. Solo/Mute buttons appear directly to the right of the instrument name. Clicking the selected mode again reverts to normal; `Esc`/clicking outside the overlay discards changes. Multiple solos are treated as OR, Mute as exclusion. Search uses case-insensitive regular expressions, space for AND, and `-condition` for exclusion, and `plugin:` condition (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases of integrated patches can also be searched. Invalid regular expressions result in an error, stopping application, random selection, and condition saving. If 0 entries, you can revise conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 entries. Role, Preset, and search string are saved on exit and restored on next launch. Presets are restored by name; if a condition is deleted, it reverts to `ALL` for that Role. If the selected Role becomes 0 entries during restoration or Solo/Mute change, it reverts to `ALL` globally; if Preset becomes 0 entries, it reverts to `ALL` for that Role.
  - The list displays single-line patch name cells without wrapping, arranged in multiple columns from left to right, then to the next row, adjusting to the window width. In narrow windows, it reverts to a single column. The selected patch is maintained upon resize, and changing only the number of columns does not trigger a reload. Clicking a patch does not move the display position of the list. Arrow keys or `h`/`j`/`k`/`l` move as per the visual layout. Left/right move to the previous/next patch (to the end of an adjacent row at row boundaries), and up/down move to the rows above/below, moving to the end of a short final row if no patch is directly below. Inputting a number first, like `2j` or `10k` (Vim-style), moves that many times (stops at boundaries; numbers are canceled by non-movement keys). `PageUp`/`PageDown` move by 10 rows up/down, and combined with a number, move by that multiple. Key movement, Random, filtering, and column count changes scroll the selected row into view only if it's off-screen. If all patches fit, no scrolling occurs. For single instrument results, the instrument name is omitted; for mixed results, it's retained. Text color indicates only one difference within the results. If instruments are mixed, the instrument name part is color-coded per instrument. For a single instrument, if Preset is `ALL`, patch names are color-coded per corresponding Preset; if not distinguishable, then per category. Colors are Monokai-based. If all are the same, normal text color is used. Colors cycle through 5 hues, so distant, different groups might have the same color. For 'pink' patches, 'pink' is prioritized in the patch name. Classification folders etc. are omitted, and necessary paths are retained if distinguishing identically named patches. Dexed program specification is maintained. Hovering over displays the full display path, source path, active categories, and merged count if 2 or more. Load/Size are not permanently displayed in the list, nor are `no category`/`1 merged`. Regular patches immediately request application when selection changes via mouse or key movement; the same behavior occurs when conditions change and a different candidate is selected. Opening does not change the patch. Upon opening, if the patch currently playing (applied via Browse patches) is in the current results, that patch is selected and scrolled into view (no re-application). The currently playing patch is saved on exit and inherited as the restored patch on next launch. Adding/deleting effects or browsing effects maintains this inheritance. If the playing patch is not in the results or unknown, the selection from when it was last closed is retained (unselected when opened for the first time after launch). After changing patches via Favorites/History, instrument loading/deletion, the playing patch is unknown, and selection is also cleared. Key movement starts from the selected patch when opened; if unselected, it moves to the beginning. The next selection can be made even during preparation, and pending requests are replaced by the latest one. Upon normal successful application, duplicate patch names or fixed padding at the bottom are not displayed. A short guide is shown only when the selected and applied patches differ, indicating that the previous patch might continue playing for unconfirmed 'pink' patches, etc. The 'pink' Space key prompt and application failure/save errors are displayed.
  - 'Pink' patches with sample capacities of 64,000,000 bytes or more are not automatically applied (unknown capacity is treated as normal). After selection, `Space` opens a confirmation overlay for instrument, patch, and capacity. Pressing `Enter` within this overlay starts preparation and application. `Esc` during confirmation cancels without changing. Once a patch is applied, re-confirmation is still required. A progress overlay is shown during loading, and cancellation is not possible. `Space`/`Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (for Solo/Mute overlay, `Enter` is the same as `OK`).
  - The `Random` button or `R` key within the browser selects evenly from the displayed results. While typing text, shortcuts like `T`/`R`/`M`/`/`/`h`/`j`/`k`/`l`/numbers/`Enter` do not work, and `R`/`M` also do not work when an overlay or save menu is displayed within the browser. For 'pink' patches, selection alone still requires the same Space → Enter confirmation. `Esc`, `Enter`, or OS close of the browser window closes only the browser, preserving the main app and the last successfully applied patch, discarding pending requests and old preparation results. Already started application processes are completed. When confirmation overlay, Solo/Mute, or save menu is open, `Esc`/clicking outside the overlay closes only the child, and does not pass the same input to the browser or background operations. OS close closes the entire browser. It can be reopened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to a Role via `Save condition to Role` in the hamburger `☰` menu and deleted via `Delete` in Preset. When adding from ALL, the destination is Etc. These are handled by the common selector's normalization and classification rules and saved to `patch-filter-presets.json` within cat's config directory. If loading is corrupted, the original file is protected, writing is stopped, and save failures are displayed on screen. Role/Preset/Search/Solo/Mute are maintained during the same app session; only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared or migrated.
  - During Browse patches preparation, current playback continues, and conflicting load, scan, and patch operations are disabled. There may be waits for sample loading (e.g., Floe). Global MML/Velocity/CC1 selections, effect order/state/Bypass are maintained, and upon success, the state for next launch is saved. Favorites/History are not automatically updated. Application success and save failure are displayed distinctly. Preparation/application progress is updated even when child windows are focused.
  - Patch browser candidates are CLAP patches from Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, and Vaporizer2 in the existing cmrt catalog, matching the format + ID of installed instruments.
  - The patch browser catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode. VST3 alternatives and unknown instruments are not targeted. Effect preset selection uses the Browse effects in the effect routing section. Dexed retains program specifications within its cartridge. Floe/sforzando external samples are required in their original locations; there is no function to duplicate or save samples.
  - In environments without cmrt or the above `catalog.json`, Surge XT's patch list is fetched on each launch from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns), saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json`, and used as candidates for Browse patches. If fetching fails, the previously saved list is used; if that's also unavailable, the patch browser displays a load failure. A 'loading' message is shown until fetching completes (max 10 seconds). The list is generated from Surge XT 1.3.4, so increased patches in different Surge XT versions won't appear, and selecting a non-existent patch displays a preparation failure. If `catalog.json` exists but cannot be read, it does not switch. The reason is documented in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - If preparation/application fails, the reason is displayed. The original configuration, state, and playback state are restored, and instances where state restoration also failed are suppressed from saving. Save failures after patch application are displayed as applied. Instrument-specific preparation/application conditions and verification scope are documented in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - For each plugin, `★ Add favorite` allows saving multiple instances of the current patch and settings as favorites. For instruments, this also saves the playback pattern, selected items when stopped, and confirmed MML/chord text. For effects, playback settings are not saved. Names are assigned automatically and can be changed later.
  - When `Load`ing a different plugin, the patch, settings, and playback pattern of the plugin being replaced are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. It is not added on reloading the same plugin or during startup restoration. Confirmed MML/chord text and stopped selections for instruments are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is aborted.
  - During manual and automatic saving, duplicates of the same plugin, role, playback pattern, confirmed MML text, and state are checked, and existing items and names are reused. In History's automatic recording, only differences in stop state are treated as identical, but MML with different text is kept as separate items. Effects do not compare playback settings.
  - Exceptions, reasons, scope, and limitations for each product can be found in [Plugin-Specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md). For the confirmed state format of TyrellN6 CLAP, compressed parts that change only during playback are excluded from duplicate detection, and patches with identical text-based tone settings are grouped. Confirmed PCore `UI_op=9/10` and empty lines at the end of text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For the confirmed format of Vaporizer2 CLAP, tiny differences in MSEG time/coordinates that occur during loading/saving (absolute difference for specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less; see [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) are treated as identical. Edits within this margin cannot be distinguished. Saved data is kept in its original form.
  - Switch between lists using the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order, with new additions placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - Turning `Reorder favorites` ON in the Favorites hamburger menu allows sorting with `↑`/`↓` at the end of the row and immediate saving. Row click loading and effect removal work as before. `Sort by plugin name` sorts and saves once in ascending order by plugin name (case-insensitive, maintaining relative order for identically named items). Manual adjustments can still be made afterward. Sorting is disabled while filtering, and sorting mode is OFF at startup. See [Rationale and Specifications (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is sorted by most recent registration. If a patch is deemed identical, the existing saved patch is reused, its registration time is updated, and it is moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are maintained. All items from the old format are migrated to History, and all items not starting with `auto ` are also kept in Favorites (because the old format did not record the origin of automatic saves, all renamed automatic saves are included in History). See [Policy and Reasons (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus. Renaming and deletion are performed in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If a CLAP plugin with the same name, manufacturer, and type exists, VST3 plugins in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are displayed). Turning it OFF shows both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and maintained on next launch. Favorites and the format used for loading/restoring are not changed.
  - In both tabs, clicking the name/image loads it, and for connected effects, clicking again removes them. The currently loaded/connected row is highlighted, and the color is maintained even when an effect is bypassed. The same operations can be performed with the `Load`/`Add`/`Remove` buttons in Plugins.
  - Favorites are displayed one item per line. Long names and supplementary information are truncated; hovering over them reveals the full text. Waveforms and spectrum are displayed below Routing on the right side; the width of the right side can be adjusted by dragging the border. `On right` in `Audio analysis` allows switching the analysis display between bottom-right / bottom of the screen (defaulting to bottom-right at startup).
  - For sforzando CLAP's confirmed state format, existing favorites are reused without creating new identical patches based solely on differences in save counter and note/CC1 performance values. Differences in patch settings are preserved. See [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` label is displayed to the left of each row. Clicking a favorite name once restores its patch and settings. For instrument favorites, the saved confirmed text and playback pattern selection are restored. While stopped, it remains stopped; `Play` plays the restored phrase. While playing, it switches to the saved pattern/MML. Favorites in the old format without text retain the current text, and old-format stopped items maintain the current selection. If conversion or state loading/connection fails, the original text/selection/playback state is restored, and the failed content is not saved to the session. Instrument favorites preserve connected effects, and effect favorites preserve the instrument and MML. An instrument is required to call up an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed, and clicking the name again removes that effect. This also applies if settings were edited after calling or during Bypass. The instrument, other effects, and saved favorites remain. Another patch of the same format + ID switches settings at the current position, and unconnected effects are appended to the end. The selected favorite for each effect is also restored on next launch.
  - Rename/delete operations are available from `…` at the end of the row. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Brief audio glitches may occur during saving/restoration, and the scope of what can be saved is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` is unchecked for the oscilloscope, spectrum, oscilloscope, and Lissajous displays are arranged horizontally at the bottom of the screen.
  - A 45° rotated Lissajous displays the most recent 50ms of output (max 4096 samples), reflecting effects and Bypass. A common auto-gain for both channels maintains volume differences and aspect ratio of the figure; same-level in-phase signals are vertical lines, out-of-phase are horizontal, and sine waves with phase differences become ellipses or circles. Left-only output is a top-left to bottom-right diagonal line, right-only is a top-right to bottom-left diagonal line, mono is a vertical line, and silence is a central dot.
  - It updates independently of notes, cycle count, or trigger settings, and `Show analysis labels` can display headings. Axes and drawing methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation over the most recent 150ms as a bar and numerical value from -1 to +1. +1 indicates in-phase, near 0 indicates weak correlation, and -1 indicates out-of-phase. Negative values are shown in red, serving as a guideline for judging cancellation when mixed to mono. If silent or one channel is silent, it displays '—'.
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) output, reflecting effects and Bypass, superimposed. Mono displays the same waveform on both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and you can select `1`, `2`, `4`, or `8 cycles`. After Note Off, the decay is displayed based on the last note. Output containing chords or detuning may not perfectly repeat within that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel. If the left channel is silent, the right channel is used; if no crossing, it displays from the beginning.
  - `Similarity` sweeps the buffer from the beginning by the display width, 1 sample at a time, to find the position with the maximum correlation coefficient to the previous display. In case of a tie, the earlier position is adopted, and the same starting position is used for both left and right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when note playback, playback pattern, displayed cycle count, trigger method, or audio stream changes. A waiting display is shown until sufficient data is collected.
  - The search is performed on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - The display settings at startup are 4 cycles and zero-cross. Display settings are not saved.

# Miscellaneous

This section was generated by AI and is hard to read. I plan to revise it later.

## Window Position and Size Settings

You can specify the main window's launch position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved to `status.json` in the same directory, and regular saving does not overwrite the TOML. If states are mixed in the old TOML, they are migrated on first launch: the original file is saved as `config.toml.before-status-migration.bak`, then only state items are removed. Setting comments, formatting, and unknown items are preserved, and independent comments attached to removed items remain at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates, used as the launch position if both are specified. If omitted, it defaults to the traditional initial position. `width` and `height` are the inner width and height of the main window (logical coordinates). Either can be specified alone; omitted sides or values less than or equal to 0 or non-finite revert to 900 width and 600 height. Changes are reflected on next launch. The plugin placement method is currently `right_then_bottom_right` only; omitting it results in the same behavior. GUI size and placement are calculated based on the actual window frame, screen working area, and DPI. Automatic saving of manually moved main window/GUI positions and per-plugin position specification are not performed.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, and execute from this app's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` always refer to local paths from Cargo.toml. Play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to the traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` requires their re-export. The patch browser uses the public Role/Preset/filter API and tui-core's sample capacity determination. Browse effects utilize play-server's `AudioEffectCatalog` / `EffectRenderer`, TUI's effect selector's selection conditions, and tui-core's search conditions. Insufficient checkout or missing APIs will result in a build error, and there will be no automatic switch to Git versions.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is located outside the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The UAPMD location during build is specified by an environment variable. `config.toml` is not read as a build setting.

Do not use `cargo install`; directly run the exe generated in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require reconstruction and rebuild. If `target/shim` was created with a different version like VS 2022, it will continue to build with that version, so please delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe and unpacked to a cache directory for loading when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency structure and operational policy. TUI local switching is not required for building with cat. If cross-building TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure, with manual handling for 'off'. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. When verifying, record the HEAD and diff of each repo.

Launch the GUI without arguments. `--help` and `--version` are also available.

# For Developers

## Recording

When reproducing first-time setup for an introductory recording, use `cargo build --release --features recording-mode`. For each launch, an empty save location is created under `%LOCALAPPDATA%\cat-plugin-player-recordings\`. It does not read/write normal settings/history and detects only Surge XT/Surge XT Effects CLAP. It does not refer to the cmrt catalog and acquires a new catalog for Surge. The absence of Surge XT is not faked; it is expected that a human or coding agent will actually uninstall it before recording. After recording, revert to the normal exe with `cargo build --release`. Refer to [ADR 0026](docs/adr/0026-recording-mode.md) for scope and limitations.

## Surge XT Effects State Table

When Surge XT is updated, or similar, run the following in an environment with Surge XT Effects installed to recreate the table.

```
cargo run --release --example export_surge_fx_states -- ..\cat-music-patterns\surge-xt-fx-states.json
```

Commits/pushes to cat-music-patterns are performed manually. See [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for format and reasons.

# Future Brainstorming
- ※Subject to change based on whim.
- Extend the strengths not found in cmrt, such as VST3 and GUI support.
  - Integrate GUI edit result status with cmrt.
    - Load and use edited patches and effects in cmrt.
  - Experiment with egui.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, playing multiple SMFs together, swapping patches and effects?
    - Should these be split into separate apps? Or is it better to do them in this app given patch and effect swapping? What kind of workflow is envisioned?
- Utilize various cmrt crates for different functionalities.
  - ※This also refines the crates, and we benefit from using them, so it's a win-win.
  - CLI import / export
    - The cmrt keyboard screen can play patches, effects, arpeggios, etc., with a single command from CLI. Aim to enable the same (to a level where it plays by just changing cmrt to cat-plugin-player).
      - Frequent breaking changes are acceptable. Manual migration is possible later. Better than halting progress by fearing changes.
  - SMF import / export
    - Currently, output with audio plugin name and patch name recorded in SMF is implemented; this will be further developed.

# Concept, Goals
- Instrument
  - For example, a synthesizer with a MIDI keyboard makes a sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations required to make a sound.
- Sound Production
  - We aim to maintain 'it sounds in the author's environment' as much as possible.
- Instant Sound
  - Perceptibly 0.2 seconds from app launch to sound. Playback is optimized for speed.
    - Processes like GUI display preparation are performed after the sound starts.
      - Sound playing before GUI display might be considered a sound version of a splash screen.
    - You need to run the release-built exe directly (running via cargo involves timestamp checks, and debug builds have debug code, both of which add to startup time).
- Educational Use
  - It is oriented more towards educational use than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes are made.

# Random Thoughts Corner
- ※Related to the concept.
- I like audio plugin preset patches.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It is the sum of each layer from audio plugin to PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and playback UI.
        - This is a layer where vibe coding can be applied.
        - Building this can improve UX.
- For whom is this instrument?
  - Cats, babies.
    - That means widening the target range towards beginners to include cats and babies (e.g., on Android tablets).
  - ※This is a metaphorical expression. Actual operation on Android tablets is unconfirmed. The likelihood of efforts to make it work on Android if it doesn't is low.

# Out of Scope, Not Goals
- Robustness
  - Absolutely no bugs no matter what. The app never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Usability
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation without CPU load in any environment. Audio never drops out.
- Requests
  - Immediately responding to all user requests.

# License
- MIT