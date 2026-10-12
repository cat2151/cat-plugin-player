# cat-plugin-player

A lightweight application that allows you to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- The sound quality of audio plugins is superb!
- To play them, you often need to launch a DAW and perform preliminary setup.
- Lightweight apps for easy playback sometimes required account registration to obtain, or their operation felt 'not for me'.
- Nowadays, if you have a desired UX, you can achieve it through 'vibecoding'.
- So I decided to do just that.

# Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

# Demo Video: Automatic Audio Plugin Installation, Patch & Effect Browsing

If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as shown. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as mentioned above.
- Detection
  - Upon startup, it automatically detects installed plugins.
- Playback
  - Clicking on a plugin will load it and automatically start playback.
  - It can play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit patches and effects.
- Patches
  - Each plugin has a variety of preset patches. You can easily select them using a unique patch browser (supported by some plugins only).
  - For plugins not supported by the patch browser, you can select patches using the plugin's GUI as usual.
- Effects
  - Each plugin has a variety of preset effects. You can easily select them using a unique effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects using the plugin's GUI as usual.

※↑This section is written by a human. AI is prohibited from writing here.

# AI-Edited Section

It's hard to read, so I'll fix it later. Folding it for now.

<details><summary>Feature Details and More</summary>

## Features (Detailed additions by AI)

The display method and constraints for the oscilloscope are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Detection
  - If plugin detection succeeds but no instrument is found, a prompt to install the free Surge XT via winget is displayed. It also displays if only effects are found. If Windows display language is Japanese, it will be in Japanese; otherwise, in English. Even if "Later" is chosen, it will be displayed on every startup as long as no instrument is available.
  - "View Terms of Service" opens Surge XT's terms (GPLv3) in a browser. Pressing "Agree to Terms and Install" executes winget, showing stages "1/4 Preparing", "2/4 Downloading", "3/4 Installing", "4/4 Loading" with elapsed seconds. If a Windows confirmation screen (User Account Control) appears at 3/4, select "Yes". Upon success, it automatically rescans the plugin list, loads Surge XT (CLAP), and starts playback like `Load`. In this case, previous instrument/effect restoration is not performed. Even if the prompt is closed, installation continues, followed by rescan and loading upon completion.
  - If winget is unavailable or installation fails, winget's message and manual download links are shown, and recorded in `error.log`. Only Surge XT is included, and download progress is not displayed. The reason for this choice is in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- GUI
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, then automatically takes a screenshot, and scales it down to a PNG with the correct aspect ratio, fitting the list display frame and screen zoom level. It is saved in `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers share the same image across VST3 / CLAP (ignoring leading/trailing spaces and case). If name/manufacturer are missing, they are distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated to shared images based on list/favorite identification information. Patch state and favorite load destinations are still distinguished by format + ID.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while maintaining aspect ratio. Hovering over an image shows it at its saved size. Plugins with no GUI displayed show a temporary icon of the same height.
  - The GUI prioritizes positions adjacent to the right of the main window and aligned to the top, fitting within the screen's working area. If it doesn't fit, it's placed in the bottom-right of the main window's screen working area. This placement occurs when displayed and when the GUI is resized, but it doesn't constantly follow the main window's movement. The GUI is not scaled down; if its width is larger than the screen, its left edge aligns with the working area; if its height is larger, its top edge aligns.
  - For blank images or failed captures, retries occur at 1-second intervals for up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured and replaced with a valid image when the GUI is opened. No re-captures occur after success.
  - If capturing fails after retries, an error is displayed. Opening the GUI again retries. For GUIs not supporting capture, the temporary icon remains. To re-capture or if the saved PNG is corrupted, delete the shared PNG for that plugin and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are kept during migration).
- Playback
  - The last played instrument, connected effects, Bypass status, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on the next startup. Stopping playback pattern selection and confirmed MML/chord text are also restored. If there is only one instrument candidate in the Plugins list on startup, it will be automatically loaded if not already loaded, and playback will start with the selected pattern even if saved in a stopped state. If there are multiple candidates, the stopped state is restored. If manually stopped after startup, it remains stopped. Uncommitted edits are not saved.
  - CLAP Note On/Off is sent according to the input format supported by the plugin. This includes sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or a dropdown. The arrows cycle at both ends of the types, and stop is not included in the types. Switching during playback releases all notes of the current phrase and starts from the beginning of the new phrase. When stopped, selecting a type does not play; `Play` starts it, `Stop` stops it. In the main window, the `Space` key also toggles start/stop (except when typing). Playback pattern selection including MML/chords and stop state are restored on next startup. However, if only one instrument candidate exists on startup, it auto-plays from the stopped state.
  - To preview MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). An example of single notes is `t120 c8r8d4e4r4`, and a chord progression is `C F G C`. Both are accepted in the same input field; chord notation conversion is attempted first. Capital `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter`/`Confirm` and canceled with `Esc`/`Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are used for typing, and `Space` for play/stop does not work.
  - Confirmed phrases loop playback with `Play`/`Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses overall normal playback. If playing just before opening, after confirmation, the new phrase plays from the start; after cancellation, the original playback pattern plays from the start. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to regular playback patterns. Immediately after confirmation, the text and selection/playback state are saved; cancellation also saves the reverted playback state. These are restored on next startup or from instrument favorites/History. Uncommitted text is not saved, and cancellation/conversion failure retains the previous confirmed text.
  - When changing input or moving the cursor, the single note/chord at that position is previewed once. This also applies to mid-edit and paste. Moving within the same phonetic unit or re-drawing does not re-trigger sound. Previous tempo, octave, and note length are reflected, and no sound is produced at rests or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the preview sound ends. The preview sound is released upon the next phonetic unit, cancellation, confirmation, or instrument change. Previewing does not change confirmed text or normal selection.
  - For loop playback of confirmed MML, the `Velocity` field's setting takes precedence over any in-input specification. CC1 can also be manipulated from the `CC1 modulation` field. If the playback event capacity is reached with a very dense phrase, active notes are released and sound production stops. You can restart with `Stop` -> `Play`.
  - When calling from app Favorites/History or changing patches via Browse patches/Browse effects, the stop/play state is maintained. During playback, the phrase plays from the beginning, not inheriting any waits from mid-phrase before the change. Leading rests specified in MML are preserved. While the patch browser/effect browser is displayed, `Space` for play/stop and MML input initiation are suppressed. Patch changes in the plugin's own GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is reconstructed due to loading, state restoration, connection changes, etc., playback waits for the first phrase to start while continuing normal audio processing, to preserve the initial sound. At 48kHz, this wait is about 21ms for Six Sines and 85ms for TyrellN6. MML's leading rests add to this. For application conditions and actual machine verification, refer to [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases values in Note On order within the phrase, then decreases them in the next loop, repeating this behavior. Changes are reflected from the next note without restarting the phrase, and are restored on next startup.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or a dropdown. The initial value is 0. `sweep` repeats a sweep from 0 to 127 in 2 seconds, then 127 to 0 in the next 2 seconds. The selection is also restored on next startup.
  - Guitar open strings send MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) sequentially with Note On at 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Off simultaneously, repeating after 0.5 seconds.
  - On `Load` success, it is remembered when selecting a `Sequence` type, starting, or stopping. Memory persists even after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, binary states such as patches and parameters exposed by plugins via UAPMD's state API are saved per plugin in `%LOCALAPPDATA%\cat-plugin-player\states\`. On next startup or re-loading the same plugin, these are restored before playback begins.
  - `Load` for an instrument replaces only the instrument, maintaining connected effects and settings. The state of the plugin being replaced is saved, and it switches after successful generation, restoration, and connection of the new plugin. If it fails, the original configuration is maintained and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. This does not include duplication of external samples or saving upon forced termination.
  - On startup, the previous instrument and effect are loaded directly, and their states are restored before playback begins. The GUI is then initialized, and the list is scanned in the background.
  - If direct loading is not possible due to old history or file movement, it is restored from the scan results after GUI display. Old history is updated once played.
  - If the main instrument is CLAP Floe, when audio processing is rebuilt due to loading or patch restoration, the start of the first phrase is delayed by 100ms. MML's leading rests are further added. This is a provisional measure for observing the symptom of missing initial sound, and actual machine resolution is unconfirmed. For application conditions and review policy, refer to [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md).
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - When `Browse effects` / `E` / `X` is opened, if there's only one corresponding CLAP effect candidate, it's automatically loaded, and its slot becomes the target for preset changes. If already connected, it's not reloaded, and that slot is used. If no instrument is loaded and there's only one instrument candidate, the instrument is loaded first and playback begins. If there are multiple instrument candidates, select an instrument first. The candidate count determination follows the CLAP priority setting in the Plugins list and does not reduce the candidate count based on search strings.
  - The connection is a serial chain: one instrument -> multiple effects -> output. Load an instrument, then press `Load` for the first effect, and then `Add` to append subsequent effects. `Remove` for a connected effect removes only that one. Duplicate effects with the same format + ID cannot be connected.
  - Changing effects via Browse effects requires an instrument. Candidates are retrieved from default effect/preset locations explored by the play-server shared core; no fallback to VST3 occurs. During preparation, current playback continues, and loading, scanning, and patch operations are disabled. While stopped, it remains stopped; while playing, the current phrase restarts from the beginning, maintaining overall MML/Velocity/CC1 selection and other effects' order/settings/Bypass. Upon generation/application/connection failure, it reverts to the original configuration. Upon success, the applied state and connection are saved for next startup; Favorites/History are not automatically added. Save failures are displayed as applied.
  - The `Browse effects` button next to `Sequence` or the `E`/`X` key opens a separate maximized effect browser window where presets can be selected from a list. It cannot be opened while MML is being edited. Narrow down candidates by Category -> Kind on the left and by search. Classification and selection targets are the same as cmrt's effect selector. Search uses case-insensitive regex, space for AND, `-condition` for exclusion, applied to preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regex during input shows an error and retains the previous list. The top row displays Search, Target Slot, and `Random`. The search field uses remaining width, and `/` within the browser moves focus to the search field. `R` randomly selects from narrowed results. Multi-column display, navigation with arrow keys / `h`/`j`/`k`/`l` / number prefixes / `PageUp`/`PageDown`, and closing with `Enter`/`Esc` are the same as Browse patches. For results mixing multiple effects, effect names are displayed per effect; if Kinds differ, preset names are color-coded per Kind.
  - The target slot is chosen from `Add new effect` or `Replace n` of a connected effect. When opened, it defaults to the last slot written to by this browser, otherwise the last effect, or `Add new effect` if no effects exist. Applying with `Add new effect` makes the added effect the target, and subsequent selections replace that slot, so the number of effects doesn't increase by continuously selecting. If the target slot is removed, the last effect becomes the target. Presets of connected effects in non-target slots are greyed out, and selecting them displays a reason and does not change them. Presets put into the target slot by this browser are marked with `▶`, and opening the browser selects them (does not reapply).
  - Clicking/keying, or narrowing down to another candidate, immediately requests application. Opening alone does not change the effect. You can select the next even while preparing; unstarted selections are replaced by the latest one, and canceled when closing. Category, Kind, search string, list selection position, and target slot are saved on exit and restored on next startup. Key navigation starts from the restored selection. Category/Kind are restored by name; if missing, they revert to `ALL` in their field. Selection is restored by preset; if it's no longer in the catalog, it becomes unselected, and key navigation starts from the top. The target slot is restored by its position in the chain; if no effect exists at that position, it defaults to the slot it would have on opening.
  - If the effect before or after replacement in Browse effects is CLAP Surge XT Effects, the sound just before switching fades out over approximately 50ms, and the sound after switching fades in over approximately 50ms. This is a provisional measure to suppress a loud "pop" if residual sound from the instrument passes through the new effect (especially Distortion types). Switching is delayed by that amount (max ~70ms), and residual sound may be faintly audible during fade-in. No fading occurs for switches between other effects or for patch/instrument changes. Refer to [ADR 0027](docs/adr/0027-surge-xt-effect-swap-fade.md) for reasons and scope.
  - CLAP Surge XT Effects presets use a table of pre-prepared states obtained on each startup from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) instead of temporarily launching the plugin to prepare them (saved in `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-fx-states.json`). This avoids a bug where the app crashes if you keep selecting presets. If it cannot be obtained, it uses the previously saved table; if that's also unavailable, Surge XT Effects presets do not appear as candidates in the effect browser, and a reason is displayed. The table is created from Surge XT 1.3.4 presets; presets with different contents (e.g., different Surge XT versions) are excluded from candidates, and the number of excluded items is shown. Refer to [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for reasons.
  - `Routing` displays the connection order. You can reorder effects by dragging the `↕` button up or down. While dragging, the moving effect name and insertion line are displayed. Instruments cannot be reordered. `Show UI` for each plugin allows editing patches and effects. `Instrument`/`Effect` are displayed with the same type color as the left list.
  - Removing all effects reverts to direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing passthrough while retaining instance and settings. It can also be toggled with the `b` key (does not work in MML input screen, Browse patches/Browse effects, or while typing). While bypassed, connected effects in Routing are greyed out and `Bypassed` is appended.
  - `Sequence` is for instruments. Stopping `Sequence` continues audio processing to allow effect decay.
  - On next startup, the instrument, order of all effects, their respective states, and Bypass status are restored before playback. History of 1 old format effect can also be loaded. If effect restoration fails, an error is displayed, it reverts to direct instrument connection, and the saved configuration is retained.
  - Add/Remove/reorder are saved only if the new connection is successfully prepared. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono -> stereo is duplication, stereo -> mono is averaging L/R. Auxiliary inputs are silent.
  - Changes to connections or Bypass may cause brief audio glitches, and active notes and decay may be cut off. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Patches
  - If no instrument is loaded and there's only one instrument candidate, `Browse patches` / `T` automatically loads it and starts playback. Requests during loading/scanning are processed after completion; closing the list cancels unstarted automatic loads. If multiple candidates exist, it does not auto-select. Candidate count determination follows the CLAP priority setting in the Plugins list and does not reduce candidate count based on search strings. Load failures are displayed, and no automatic retries occur. Patch list supported formats and catalog limitations are as before.
  - The `Browse patches` button next to `Sequence` or the `T` key opens a separate maximized patch browser window on the screen where the main window resides. It does not change the main window's position/size. It cannot be opened while MML is being edited. Narrow down candidates by Role -> Preset -> Search with frames. Roles/Presets with no patches are not displayed. Items that result in 0 items after Solo/Mute (e.g., `FM Bass` when Soloing everything except Dexed) are also hidden. Search field strings do not hide items; custom conditions are displayed even if 0 items, to allow deletion. The left condition area and right Patches list are individually scrollable. It uses remaining width and height for the list, allowing operation even when unmaximized or in a narrow window. The top row displays Search, `Solo/Mute`, `Random`, `☰`. The search field uses remaining width, and `/` within the browser moves focus to the search field. Detailed instrument conditions can be checked by hovering over `Solo/Mute`. Clicking this button or pressing `M` within the browser opens a vertical Solo/Mute overlay, confirmed collectively by `OK` or `Enter`. Solo/Mute appears immediately to the right of the instrument name. Clicking the selected mode again reverts to normal; `Esc` / clicking outside the overlay discards changes. Multiple Solos are treated as OR, Mute as exclusion. Search uses case-insensitive regex, space for AND, `-condition` for exclusion, and `plugin:` condition (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases for integrated patches can also be searched. Invalid regex results in an error, stopping application,抽選, and condition saving. If 0 items, you can re-examine conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 items. Role, Preset, and search string are saved on exit and restored on next startup. Preset is restored by name; if the condition is deleted, it reverts to `ALL` for that Role. If the selected Role becomes 0 items during restoration or Solo/Mute change, it reverts to `ALL` for the entire list; if Preset becomes 0 items, it reverts to `ALL` for that Role.
  - The list displays patch name cells on a single, non-wrapping line, arranged in multiple columns from left to right, then to the next line, depending on width. In a narrow window, it reverts to 1 column. Resizing maintains the selected patch, and changing column count alone does not reload. Clicking a patch does not move the list's display position. Up/down/left/right arrow keys or `h`/`j`/`k`/`l` move as they appear visually. Left/right move to the previous/next patch (to the end of the adjacent line at line ends); up/down move to the row above/below; to a short last row with no patch directly below, it moves to the end of that row. Entering a number first, like `2j` or `10k` (Vim-style), moves that many times (stops at the edge; numbers are canceled by non-movement keys). `PageUp`/`PageDown` move by 10 rows up/down; combined with numbers, they move by that multiple. Key navigation, Random, narrowing, and column count changes only scroll to make the selected row visible if it's off-screen. If all patches fit, it does not scroll. For single-instrument results, the instrument name is omitted; for mixed results, it's kept. Text color indicates only one difference within results. If instruments are mixed, the instrument name part is color-coded per instrument. For single instruments, if Preset is `ALL`, patch names are color-coded per relevant Preset; if that doesn't distinguish, then per category. Colors are Monokai-based. If all are the same, normal text color is used. Colors cycle through 5, so distant groups might have the same color. Pink patches prioritize pink text. Classification folders etc. are omitted; if distinguishing same-name patches, necessary paths are kept. Dexed program specification is maintained. Hovering shows the full display path, original application path, active category, and count of 2+ merged items. Load/Size are not permanently displayed in the list; `no category`/`1 merged` are also not shown. Normal patches immediately request application when selection changes via mouse click/key navigation; same behavior when another candidate is selected by condition change. Opening alone does not change the patch. If the patch applied and playing via Browse patches is in the current results when opened, that patch is selected and scrolled into view (no re-application). The playing patch is saved on exit and inherited as the restored patch on next startup. Adding/removing effects or Browse effects maintains this inheritance. If the playing patch is not in results or unknown, the selection from the last close is kept (unselected when opened for the first time after startup). After changing patches by loading/deleting Favorites/History/instruments, the playing patch is unknown, and selection is cleared. Key navigation starts from the selection when opened; if unselected, it moves to the top. Next selection is possible even during preparation; unstarted requests are replaced by the latest one. Upon successful normal application, duplicate patch names or fixed padding at the bottom are not displayed. Only when selected and applied patches differ is a short guide displayed, indicating that the previous patch might continue playing for unconfirmed pink, etc. Pink Space guide and application failure/save errors are displayed.
  - Pink patches with sample capacity of 64,000,000 bytes or more are not automatically applied (unknown capacity is treated as normal). After selection, `Space` opens a confirmation overlay for instrument, patch, and capacity; pressing `Enter` within it starts preparation/application. `Esc` during confirmation cancels without changing. Once applied, re-confirmation is still needed. During loading, a progress overlay is shown, and cancellation is not possible. `Space`/`Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is same as `OK`).
  - `Random` or the `R` key in the browser selects evenly from displayed results. When typing, `T`/`R`/`M`/`/`/`h`/`j`/`k`/`l`/numbers/`Enter` shortcuts do not work, nor do `R`/`M` while an overlay or save menu is displayed in the browser. For pink patches, only selection occurs, requiring the same Space->Enter confirmation. `Esc`, `Enter`, or OS close of the browser window closes only the browser, preserving the main window and the last successfully applied patch, discarding unstarted requests and old preparation results. Already started processes for application are completed. `Esc`/click outside overlay when confirmation overlay, Solo/Mute, or save menu is open closes only the child, not passing the same input to the browser or background operations. OS close closes the entire browser. It can be re-opened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to a Role via `Save condition to Role` in the hamburger `☰` menu, and deleted via `Delete` in Preset. The destination for adding from ALL is Etc. It is handled by common selector normalization/classification rules and saved in `patch-filter-presets.json` within the app's config directory. If loading is corrupted, the original file is protected, writing is stopped, and save failures are displayed on screen. Role/Preset/Search/Solo/Mute are maintained during the same app session; only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared/migrated.
  - During Browse patches preparation, current playback continues, and conflicting load, scan, and patch operations are disabled. It may wait during sample loading for Floe, etc. Overall selection of MML/Velocity/CC1, and effect order/state/Bypass are maintained; upon success, state for next startup is saved. Favorites/History are not auto-added. Application success and save failure are displayed distinctly. Progress of preparation/application updates even when child window is focused.
  - Patch browser candidates are CLAP patches from Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, and Vaporizer2 present in existing cmrt catalogs, whose instrument and format+ID match installed ones.
  - The patch browser catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode. VST3 fallback and unknown instruments are excluded. Effect preset selection uses the Browse effects in the effect routing section. Dexed retains program specification within the cartridge. Floe/sforzando external samples are required at their original locations; there is no function to duplicate/save samples.
  - In environments without cmrt and where the above `catalog.json` does not exist, Surge XT's patch list is obtained on each startup from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) and saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json` for use as Browse patches candidates. If it cannot be obtained, the previously saved list is used; if that's also unavailable, the patch browser displays a loading failure. It displays "loading" until acquisition finishes (max 10 seconds). The list is generated from Surge XT 1.3.4; for different Surge XT versions, new patches may not appear, and selecting a missing patch displays a preparation failure. It does not switch if `catalog.json` exists but cannot be read. Reasons are in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - If preparation/application fails, the reason is displayed. The original configuration, state, and playback state are restored; instances where state restoration also failed suppress saving. Save failures after patch application are displayed as applied. Instrument-specific preparation/reflection conditions and verification scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - `★ Add favorite` for each plugin allows saving multiple favorites of the current patch and settings. For instruments, playback patterns, stopped selections, and confirmed MML/chord text are also saved. For effects, playback settings are not saved. Names are automatically generated and can be changed later.
  - When loading a different plugin with `Load`, the patch, settings, and playback pattern of the replaced plugin are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. It is not added for reloading the same plugin or restoring on startup. Confirmed MML/chord text and stopped selections for instruments are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is aborted.
  - During manual and automatic saving, it checks for duplicates of the same plugin, role, playback pattern, confirmed MML text, and state, reusing existing items and names. For History's automatic recording, differences only in stop state are treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - [Plugin-specific processing policies/list (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides exceptions, reasons, scope, and limitations for each product. For TyrellN6 CLAP's confirmed state format, compressed parts that change only with playback are excluded from duplicate detection, and patches with the same textual patch settings are grouped. Confirmed PCore `UI_op=9/10` and empty lines at the end of text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For Vaporizer2 CLAP's confirmed format, minute differences in MSEG time/coordinates arising from load/save (absolute difference for specified items: time <= `0.00011` ms, normalized coordinates <= `0.000001`. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as original.
  - Switch lists with the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order; new additions are placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON allows reordering with `↑` / `↓` at the end of the row, saving immediately. Row click loading and effect removal behave as before. `Sort by plugin name` sorts once in ascending order of plugin names (case-insensitive, same-name items maintain relative order) and saves. Manual adjustments are still possible afterward. Reordering is not possible while filtering, and reorder mode is OFF on startup. See [ADR 0017](docs/adr/0017-manual-favorites-order.md) for adoption background and specifications.
  - History is ordered by most recent registration. If the same patch is detected, the saved patch is reused, its registration time is updated, and it moves to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are maintained. All old format items are migrated to History; items not starting with `auto ` are also kept in Favorites (because old format does not record the origin of auto-saves, thus including all renamed auto-saves in History). Refer to [ADR 0016](docs/adr/0016-history-instead-of-automatic-favorites.md) for policy and reasons.
  - History does not display Rename/Delete operation menus. Renaming/deletion is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If there's a CLAP plugin with the same name, manufacturer, and type, VST3s in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are displayed). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on next startup. It does not change Favorites or the format loaded/restored.
  - In both tabs, clicking name/image loads; clicking again on connected effects removes them. The currently loaded/connected row is highlighted, and the color persists during effect bypass. The `Load` / `Add` / `Remove` buttons in Plugins perform the same operations.
  - Favorites are displayed one item per line. Long names/additional info are truncated; hovering reveals the full text. Waveform/spectrum are displayed below Routing on the right; the right-side width can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display between bottom-right / bottom of screen (bottom-right on startup).
  - For sforzando CLAP's confirmed state format, it does not create new identical patches based solely on save counter differences or note/CC1 playback value differences, instead reusing existing favorites. Differences in patch settings are preserved. Refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` is displayed to the left of each row. Clicking a favorite name restores its patch/settings. For instrument favorites, it restores the saved confirmed text and playback pattern selection. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites in old formats without text retain the current text; old format stopped items retain the current selection. If conversion or state loading/connection fails, the original text/selection/playback state are restored, and failed content is not saved to the session. Instrument favorites retain connected effects; effect favorites retain the instrument and MML. Calling an effect requires an instrument.
  - For connected effect favorites, `Connected - click again to remove` is displayed; re-clicking the name removes that effect. This applies even after editing settings or during Bypass. The instrument, other effects, and saved favorites remain. Another patch of the same format+ID switches settings at the current position; unconnected effects are added to the end. Each effect's selected favorite is also restored on next startup.
  - Rename/Delete are available from `…` at the end of the row. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Short audio glitches may occur during saving/restoration, and the savable range is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` for the oscilloscope is unchecked, spectrum, oscilloscope, and Lissajous appear side-by-side at the bottom of the screen.
  - The Lissajous display shows the most recent 50ms of output (up to 4096 samples), reflecting effects and Bypass, rotated 45°. A common auto-scaling factor for both channels maintains volume differences and the aspect ratio of the figure; in-phase signals of the same level form a vertical line, out-of-phase signals form a horizontal line, and sine waves with phase differences form ellipses or circles. Left-only output appears as a diagonal line from top-left to bottom-right, right-only as top-right to bottom-left; mono is a vertical line, and silence is a central dot.
  - It updates independently of note, cycle count, or trigger settings, and `Show analysis labels` displays headings. Axes and drawing methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the correlation between left and right channels for the most recent 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 is weak correlation, -1 is out-of-phase. Negative values are shown in red, serving as a guideline for judging cancellation when mixed to mono. It shows "—" if silent or one channel is silent.
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen displays L (green) and R (blue) outputs, reflecting effects and Bypass, superimposed. Mono displays the same waveform on both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and you can select `1`, `2`, `4`, `8 cycles`. After Note Off, the decay is displayed based on the last note. Output containing chords or detuning may not repeat perfectly within that width.
  - `Zero cross` aligns with the negative to positive crossing of the left channel. If the left channel is silent, it uses the right channel; if no crossing, it displays from the beginning.
  - `Similarity` sweeps through the buffer from the start for the display width, sample by sample, to find the position with the maximum correlation coefficient to the previous display. Ties prefer the earlier position, and the same starting position is used for both left and right. For the first display or if the comparison target is silent, zero crossing is used.
  - The comparison history is reset when a note is played, the playback pattern changes, the display cycle count changes, the trigger method changes, or the audio stream changes. It will show a waiting display until sufficient data is collected.
  - Exploration is performed on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - The default display settings on startup are 4 cycles and zero cross. Display settings are not saved.

## Settings: Window Position and Size

You can specify the main window's startup position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the application. Session and favorite selection states are saved in `status.json` in the same directory, and regular saves do not modify the TOML. If states are mixed in an old TOML, they are migrated on first startup, the original file is saved as `config.toml.before-status-migration.bak`, and only state items are removed. Comments, formatting, and unknown items in the settings are preserved, and independent comments attached to removed items are kept at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates, used as the startup position if both are specified. If omitted, it defaults to the traditional initial position. `width` and `height` are the inner width and height of the main window (logical coordinates). Either can be specified; if omitted or less than or equal to 0 or non-finite, they revert to width 900 and height 600. Changes are reflected on next startup. The plugin placement method is currently only `right_then_bottom_right`; it behaves the same if omitted. GUI size and placement are calculated based on the actual window frame, screen working area, and DPI. Automatic saving of positions moved manually for the main window or GUI, and per-plugin position specification are not implemented.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this app's directory. `app` / `patches` / `patch-select` / `tui-core` from TUI and `core-lib` / `server-config` from play-server are always locally referenced from Cargo.toml. Play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` needs their re-export. The patch browser uses public Role/Preset/filter APIs and tui-core's sample capacity determination. Browse effects uses play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions, and tui-core's search conditions. Insufficient checkout or API will result in a build error; no automatic switching to Git versions is performed.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The location of UAPMD during build is specified by an environment variable. `config.toml` is not read as build settings.

Do not use `cargo install`; run the exe generated directly in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require reconstruction and rebuild. If `target/shim` was created with a different version (e.g., VS 2022), it will continue to build with that version, so delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe and unpacked to the cache directory and loaded on GUI startup.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency configuration and operation policy. Local switching for TUI is not necessary for building with cat. For cross-repository building with TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; `off` is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. When verifying, record the HEAD and diffs of each repository.

Launches the GUI without arguments. `--help` and `--version` are also available.

## For Developers

### Recording

When reproducing the first-time setup for an introductory recording, use `cargo build --release --features recording-mode`.
On each launch, it creates an empty save location under `%LOCALAPPDATA%\cat-plugin-player-recordings\`,
does not read/write regular settings or history, and only detects Surge XT / Surge XT Effects CLAP.
It does not reference the cmrt catalog but acquires a new catalog for Surge.
The absence of Surge XT is not faked; the operational procedure assumes actual uninstallation by a human or coding agent before recording.
After recording, revert to the regular exe with `cargo build --release`.
Refer to [ADR 0026](docs/adr/0026-recording-mode.md) for scope and limitations.

### Surge XT Effects State Table

When updating Surge XT, etc., run the following in an environment with Surge XT Effects installed to regenerate the table:

```
cargo run --release --example export_surge_fx_states -- ..\cat-music-patterns\surge-xt-fx-states.json
```

Committing and pushing to cat-music-patterns is done manually. Refer to [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for format and reasons.

※↑AI, please write only in this h1 section.

</details>

# Future Brainstorming
- ※Subject to change based on mood
- Leverage the strengths not found in cmrt, such as VST3 and GUI support.
  - Integrate GUI edit results with cmrt.
    - Use edited patches and effects read by cmrt.
  - Experiment with egui.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, multiple SMFs playing together, swapping patches and effects?
    - Should these be separated into different apps? Or is it better to keep them in this app because of patch and effect swapping? What workflow is envisioned?
- Utilize various cmrt crates.
  - ※This also refines the crates, and we benefit from using them, so it's a win-win.
  - CLI import / export
    - The cmrt keyboard screen can play patches, effects, arpeggios, etc., from the CLI in one go. Aim to achieve the same (to the level where it plays by just changing 'cmrt' to 'cat-plugin-player').
      - Frequent breaking changes are acceptable. Manual migration is possible later. Better than being stuck and hindering progress for fear of change.
  - SMF import / export
    - Currently, recording audio plugin names and patch names to SMF for output is implemented; further develop this.

# Concept, What We Aim For
- Instrument
  - For example, a synthesizer with a MIDI keyboard makes sound when you turn it on and press a key.
    - We aim for this level of ease of use.
      - Minimize operations to produce sound.
- Sound Production
  - Aim to maintain "it sounds on the author's environment" as much as possible.
- Immediate Sound
  - Feel the sound within 0.2 seconds from app launch. Maximize speed to playback.
    - Processing like GUI display preparation is done after the sound.
      - Sound before GUI display might be considered a sound version of a splash screen.
    - Requires running the release-built exe directly (via cargo adds timestamp checks, debug builds add debug code, both cause startup delays).
- Educational
  - Leans more towards educational use than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will be made.

# Casual Thoughts Corner
- ※Related to the concept
- I love audio plugin preset patches.
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide an instrument UX.
  - Not a DAW UX, but an instrument UX.
- What is an instrument?
  - It is a combination of each layer from audio plugins to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and playback UI.
        - This is the layer that can be "vibecoded".
        - Building this can improve UX.
- For whom is this instrument?
  - Cats, babies.
    - This means broadening the target range to include beginners, even to the extent of targeting cats and babies (e.g., Android tablets).
  - ※This is merely a metaphor. Whether it actually runs on Android tablets is unconfirmed. The likelihood of effort to make it run on Android if it doesn't is also low.

# Out of Scope, Not Aiming For
- Robustness
  - Absolutely no bugs, never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed with no CPU load in any environment. Sound never cuts out.
- Requests
  - Immediate response to all user requests.

# License
- MIT