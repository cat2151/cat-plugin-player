# cat-plugin-player

A lightweight app that makes playing audio plugins easy. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Challenges Addressed by This App
- Audio plugin timbres are fantastic!
- To play them, a DAW (Digital Audio Workstation) often needs to be launched, requiring prior setup for performance.
- Lightweight apps for easy playback sometimes require account registration for acquisition, or their operation felt "not for me."
- Nowadays, if you have a desired UX, you can achieve it through vibe coding.
- That's exactly what we decided to do.

# Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

# Demo Video

If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as shown here. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as mentioned above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking a plugin automatically starts playback after loading the plugin.
  - It can play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) sequences like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit sounds and effects.
- Patches
  - Each plugin has a wide variety of preset patches. You can easily select them using a unique patch browser (supported by some plugins only).
  - For plugins not supported by the patch browser, you can select patches using the plugin's own GUI.
- Effects
  - Each plugin has a wide variety of preset effects. You can easily select them using a unique effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects using the plugin's own GUI.

※↑This section is written by a human. AI is prohibited from writing here.

# AI-Edited Section

This section is hard to read; I'll fix it later. Folding it for now.

<details><summary>Detailed Features, etc.</summary>

## Features (Detailed additions by AI)

Oscilloscope display methods and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Detection
  - If plugin detection is successful but no instrument is found, a prompt to install the free Surge XT via winget is displayed. It also appears if only effects are found. If Windows' display language is Japanese, it shows in Japanese; otherwise, in English. If you choose `Later`, it will reappear every time the app starts as long as no instrument is available.
  - `View Terms of Service` opens Surge XT's terms (GPLv3) in a browser. Pressing `Agree to Terms and Install` executes winget, displaying stages `1/4 Preparing`, `2/4 Downloading`, `3/4 Installing`, `4/4 Loading`, along with elapsed time. If a Windows confirmation screen (User Account Control) appears at 3/4, choose "Yes". Upon success, the plugin list is automatically rescanned, Surge XT (CLAP) is loaded, and playback starts, similar to `Load`. In this case, the previous instrument/effect state is not restored. Even if the prompt is closed, installation continues, followed by rescan and loading upon completion.
  - If winget is unavailable or installation fails, winget's message and a manual download link are displayed, and recorded in `error.log`. Only Surge XT is included, and download progress is not shown. The reason for this choice is in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- GUI
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG image that fits the display frame of the list and the screen's zoom level while preserving its aspect ratio. It saves to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers (ignoring leading/trailing spaces and case) share the same image across VST3 / CLAP. If name/manufacturer is insufficient, they are distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated to shared images from the list/favorite identification info. Patch states and favorite load destinations continue to be distinguished by format + ID.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while preserving aspect ratio. Hovering over an image shows it at its saved size. Plugins with no GUI displayed show a temporary icon of the same height.
  - The GUI prioritizes placement to the right of the main window, aligned with the top edge, fitting within the screen's work area. If it doesn't fit, it's placed in the bottom-right of the main window's screen work area. This placement occurs on display and when the GUI is resized, but it doesn't constantly track the main window's movement. The GUI is not scaled down; if its width is greater than the screen's, its left edge aligns with the work area; if its height is greater, its top edge aligns.
  - In case of blank images or capture failures, retries occur at 1-second intervals, up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured, and replaced with valid images when the GUI is opened. No re-capture occurs after success.
  - If capture fails even after retries, an error is displayed. Opening the GUI again triggers retries. GUIs not supporting capture remain with a temporary icon. If you want to retake or if a saved PNG is corrupted, delete the shared PNG for that plugin and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are kept during migration).
- Playback
  - The last played instrument, connected effects, Bypass status, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next launch. Stopping playback pattern selection and confirmed MML/chord text are also restored. If there's only one instrument candidate in the Plugins list on startup, and it's not loaded, it's automatically loaded, and playback starts with the selected pattern even if a stopped state was saved. If there are multiple candidates, the stopped state is restored. If manually stopped after startup, it remains stopped. Unconfirmed edits are not saved.
  - CLAP Note On/Off messages are sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are in [CLAP Note format description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. Arrows cycle at the ends of the types; Stop is not included. Switching during playback releases all sounding notes of the current phrase and starts the new phrase from the beginning. When stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except when typing). Playback pattern selection including MML/chords and stop status are restored on next launch. However, if only one instrument candidate exists on startup, it auto-plays from the stopped state.
  - To audition MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). Example for single notes: `t120 c8r8d4e4r4`, for chord progression: `C F G C`. Both are accepted in the same input field, with chord notation conversion attempted first. Uppercase `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter` / `Confirm` and canceled with `Esc` / `Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are for typing; `Space` for play/stop does not work.
  - Confirmed phrases loop playback with `Play` / `Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses normal full playback. If playback was active just before opening, after confirmation, the new phrase plays from the start; after cancellation, the original playback pattern resumes from the start. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts full playback. You can also switch to regular playback patterns. The text and selection/playback state are saved immediately after confirmation; the restored playback state is saved on cancellation. This is restored on next launch or from instrument favorites/history. Unconfirmed text is not saved; cancellation or conversion failure retains the previous confirmed text.
  - Changing input or moving the cursor auditions the single note/chord at that position once. This applies to mid-edit and paste. Moving within the same pronunciation unit or redrawing alone does not re-audition. It reflects the previous tempo, octave, and note length, and does not sound at rest or command positions. Normal playback stops when editing is opened, and full playback does not start after the audition sound ends. The audition sound is released on the next pronunciation unit, cancellation, confirmation, or instrument change. Auditioning does not alter the confirmed text or normal selection.
  - For MML loop playback, the Velocity setting in the `Velocity` field takes precedence over any specified within the input. CC1 can also be controlled from the `CC1 modulation` field. If the performance event capacity is reached due to very dense phrases, sounding notes are released, and new notes stop playing. You can restart with `Stop` -> `Play`.
  - When loading from the app's Favorites/History or changing patches via Browse patches/Browse effects, the stop/play state is maintained. During playback, the phrase plays from the beginning, not inheriting waits from the middle of the previous phrase. Leading rests specified in MML are preserved. While the patch browser/effect browser is displayed, `Space` for play/stop and MML input initiation in the background are suppressed. Patch changes made via the plugin's own GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is rebuilt due to loading, state restoration, connection changes, etc., to preserve initial notes, it waits for the first phrase to start while continuing normal audio processing. The wait at 48kHz is approx. 21ms for Six Sines and approx. 85ms for TyrellN6. MML leading rests are added further. See [ADR 0019](docs/adr/0019-random-patch-all-catalog.md) for application conditions and actual machine verification.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases values in order of Note On within a phrase, then decreases in the next loop, repeating the action. Changes are applied from the next note without restarting the phrase and are restored on next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or the dropdown. The initial value is 0. `sweep` repeats a sweep from 0 to 127 in 2 seconds, then 127 to 0 in the next 2 seconds. The selection is restored on next launch.
  - Guitar open strings play MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) sequentially with 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Off simultaneously, and it repeats after 0.5 seconds.
  - Memory occurs upon successful `Load` and when selecting, starting, or stopping `Sequence` types. Memory persists after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, binary state (patch, parameters, etc.) exposed by plugins via UAPMD's state API is saved per plugin to `%LOCALAPPDATA%\cat-plugin-player\states\`. On next launch or reloading the same plugin, it's restored before playback starts.
  - `Load` for an instrument only replaces the sound source, maintaining connected effects and settings. It saves the state of the plugin being replaced, then switches after successfully generating, restoring, and connecting the new plugin. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but current UAPMD VST3 implementation only obtains component state. It does not include external sample duplication or saving on abnormal termination.
  - On startup, the previous instrument and effect are loaded directly, and playback starts after restoring the state. Then, the GUI is initialized, and the list is scanned in the background.
  - If direct loading fails (e.g., due to old history or file movement), it restores from the scan results after GUI display. Old history is updated once played.
  - If the main instrument is CLAP Floe, when audio processing is rebuilt (e.g., during load or patch restoration), the start of the first phrase is delayed by 100ms. MML leading rests are added further. This is a provisional measure for observing symptoms of missing initial notes, and resolution on actual hardware is unconfirmed. See [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - Opening `Browse effects` / `E` / `X` automatically loads the corresponding CLAP effect if only one candidate is found, and targets that slot for preset changes. If already connected, it uses that slot without reloading. If no instrument is loaded and only one instrument candidate exists, it loads the instrument first and starts playback. If multiple instrument candidates exist, please select an instrument first. The number of candidates is determined according to the CLAP preference setting in the Plugins list, and search strings do not reduce the number of candidates.
  - The connection is series: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. `Remove` for a connected effect disconnects only that one. Duplicate effects of the same format + ID cannot be connected.
  - Instrument is required for effect changes in Browse effects. Candidates are retrieved from default effect/preset locations scanned by play-server shared core; no fallback to VST3. During preparation, current playback continues, and loading, scanning, and patch operations are disabled. While stopped, it remains stopped; during playback, the current phrase restarts from the beginning, maintaining MML/Velocity/CC1 global selection and the order/settings/Bypass of other effects. If generation, application, or connection fails, it reverts to the original configuration. After success, the applied state and connection are saved for the next launch, and Favorites/History are not added. Save failure is displayed as applied.
  - `Browse effects` next to `Sequence` or the `E` / `X` keys open an independent maximized effect browser window, allowing you to select presets from a list. It cannot be opened while editing MML. Filter by Category → Kind on the left and by search. The classification and selection targets are the same as cmrt's effect selector. Search uses case-insensitive regular expressions, space for AND, `-condition` for exclusion, matching any of preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regex during input shows an error, retaining the previous list. The top row has Search, Target Slot, `Random`. `/` within the browser moves input to the search field, `R` selects randomly from filtered results. Multiple column display in the list, movement via arrow keys / `h` / `j` / `k` / `l` with numeric repeat counts, `PageUp` / `PageDown`, and closing with `Enter` / `Esc` are the same as Browse patches. In results with mixed effects, effect names are appended for each effect; if Kinds differ, preset names are color-coded by Kind.
  - The target slot is chosen from `Add new effect` or `Replace n` for a connected effect. When opened, it defaults to the slot last written to by this browser, or the last effect if none, or `Add new effect` if no effects are present. Applying with `Add new effect` makes the added effect the target, and subsequent selections replace that slot, so the number of effects doesn't increase by continued selection. If the target slot is removed, the last effect becomes the target. Presets for connected effects in non-target slots are grayed out; selecting them displays a reason and does not change them. Presets put into the target slot by this browser are marked with `▶`, and opening the browser selects them (does not re-apply).
  - Clicking or moving with keys, or selecting another candidate via filtering, immediately requests application. Simply opening does not change the effect. You can select the next one during preparation; uninitiated selections are replaced by the latest one, and canceled by closing. Category, Kind, search string, list selection position, and target slot are saved on exit and restored on next launch. Key movement starts from the restored selection. Category/Kind are restored by name; if they no longer exist, they revert to `ALL` in that field. Selection is restored by preset; if it no longer exists in the catalog, it becomes unselected, and key movement starts from the top. The target slot is restored by its position in the chain; if no effect exists at that position, it becomes the default slot when opened.
  - If the effect before or after replacement in Browse effects is CLAP Surge XT Effects, the sound immediately before the switch fades out over approximately 50ms, and the sound after the switch fades in over approximately 50ms. This is a provisional measure to prevent a loud "pop" if residual instrument sound passes through the new effect (especially Distortion-type). The switch is delayed by this amount (max approx. 70ms), and residual sound may be faintly audible during fade-in. No fading occurs during switches between other effects or changes to patches/instruments. See [ADR 0027](docs/adr/0027-surge-xt-effect-swap-fade.md) for reasons and scope.
  - Surge XT Effects presets use a table of prepared states (saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-fx-states.json`), obtained on each launch from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns), instead of temporarily launching the plugin to prepare. This avoids a bug where the app crashes if you keep selecting presets. If the table cannot be obtained, it uses the previously saved table; if that's also unavailable, Surge XT Effects presets won't appear as candidates in the effect browser, and a reason is displayed. The table is created from Surge XT 1.3.4 presets; presets with different content (e.g., different Surge XT versions) are excluded from candidates, and the number of excluded items is displayed. See [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for reasons.
  - `Routing` displays the connection order. You can change the order by dragging the effect's `↕` button up or down. While dragging, the moving effect's name and insertion line are displayed. Instruments are not subject to reordering. You can edit patches and effects using `Show UI` for each plugin. `Instrument` / `Effect` are displayed with the same type color as the left list.
  - Removing all effects reverts to a direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing sound to pass through while retaining instance and settings. It can also be toggled with the `b` key (does not work in MML input screen, Browse patches/Browse effects display, or during text input). While bypassed, connected effects in Routing are grayed out and `Bypassed` is appended.
  - `Sequence` is for instruments. Stopping the sequence continues audio processing, allowing effects to trail off.
  - On next launch, instrument, the order of all effects, their respective states, and Bypass status are restored before playback. History of 1 old-format effect can also be loaded. If effect restoration fails, an error is displayed, and it reverts to a direct instrument connection, while keeping the saved configuration.
  - Additions, removals, and reordering are saved only if the new connection is successfully prepared. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplicated; stereo → mono is an average of left and right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause a short audio dropout, and currently playing notes and tails may be cut off. Side-chain, parallel connections, and host-side Dry/Wet are not supported.
- Patches
  - If no instrument is loaded and only one instrument candidate exists, `Browse patches` / `T` automatically loads it and starts playback. Requests during loading/scanning are processed after completion; if the list is closed, uninitiated auto-loads are canceled. If multiple candidates exist, it does not auto-select. The number of candidates is determined according to the CLAP preference setting in the Plugins list, and search strings do not reduce the number of candidates. Load failure is displayed, and no automatic retry occurs. The supported formats for the patch list itself and catalog limitations remain as before.
  - `Browse patches` next to `Sequence` or the `T` key opens an independent maximized patch browser window on the same screen as the main app. It does not change the main app's position or size. It cannot be opened while editing MML. Filter by framed Role → Preset → Search. Roles/Presets with no patches are not displayed. Items that become 0 after filtering with Solo/Mute (e.g., `FM Bass` when everything but Dexed is soloed) are also hidden. They are not hidden by strings in the search field; user-defined conditions are displayed even if 0 items, so they can be deleted. The condition area on the left and the Patches list on the right are both scrollable. The list uses the remaining width and height, allowing operation even when unmaximized or in a narrow window. The top row has Search, `Solo/Mute`, `Random`, `☰`. The search field uses the remaining width, and the `/` key within the browser moves input to the search field. Details on instrument conditions can be found by hovering over `Solo/Mute`. Clicking this entire button or the `M` key within the browser opens a vertical Solo/Mute overlay, and `OK` or `Enter` confirms them all. Solo/Mute options are arranged to the right of the instrument name. Clicking the selected mode again reverts to normal; `Esc` / clicking outside the overlay discards changes. Multiple solos are treated as OR; Mute as exclusion. Search uses case-insensitive regular expressions, space for AND, `-condition` for exclusion, and `plugin:` conditions (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases for integrated patches can also be searched. Invalid regex results in an error, stopping application, random selection, and condition saving. If 0 items, you can revise conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 items. Role, Preset, and search string are saved on exit and restored on next launch. Presets are restored by name; if a condition has been deleted, it reverts to `ALL` for that Role. If the selected Role becomes 0 items during restoration or Solo/Mute change, it reverts to `ALL` for the entire list; if Preset becomes 0 items, it reverts to `ALL` for that Role.
  - The list displays patch name cells, one line not wrapped, in multiple columns depending on width, left → right → next line. In a narrow window, it reverts to 1 column. Resizing maintains the selected patch; changing only column count does not reload. Clicking a patch does not move the list's display position. Arrow keys or `h`/`j`/`k`/`l` move according to the visual layout. Left/right move to previous/next patch (to the end of an adjacent line at line ends); up/down move to adjacent lines, to the end of a short final line if no patch is directly below. Inputting a number first, like `2j` or `10k` (vim-style), moves that many times (stops at ends; numbers are canceled by non-movement keys). `PageUp`/`PageDown` move by 10 times up/down; combined with numbers, they move by that multiple. Key movement, Random, filtering, and column count changes scroll only if the selected row is off-screen. It does not scroll if all patches fit. For single-instrument results, the instrument name is omitted; in mixed results, the instrument name remains. Character color indicates only one difference within the results. If instruments are mixed, the instrument name part is color-coded per instrument. For a single instrument, if Preset is `ALL`, patch names are color-coded per relevant Preset; if that doesn't distinguish, then per category. Colors are Monokai-based. If all are the same, regular text color is used. Colors cycle through 5, so distant groups might have the same color. For "pink patches", "pink" color is prioritized. Classification folders, etc., are omitted; if distinguishing same-name patches, necessary paths are retained. Dexed's program specification is maintained. Hovering shows the full display path, source path, valid category, and merged count if 2+. Load/Size are not always in the list; `no category`/`1 merged` are also not displayed. Normal patches immediately request application when selection changes via mouse/key movement; the same behavior occurs when a different candidate is chosen due to condition changes. Simply opening does not change the patch. When opened, if the currently playing patch applied via Browse patches is in the current results, that patch is selected and scrolled into view (not re-applied). The playing patch is saved on exit and inherited as the restored patch on next launch. Adding/removing effects or Browse effects maintains inheritance. If the playing patch is not in the results, or unknown, the selection from when it was last closed is maintained (unselected when first opened after startup). After changing patches by loading/removing Favorites/History/instruments, the playing patch is unknown, and selection is cleared. Key movement starts from the selection when opened; if unselected, it moves to the top. Next selections can be made during preparation; uninitiated requests are replaced by the latest one. Successful normal application does not display duplicate patch names or fixed margins at the bottom. Only when the selected and applied patches differ is a short notice displayed, indicating that the previous patch might continue playing for unconfirmed "pink patches" etc. "Pink patch" Space prompt and application failure/save errors are displayed.
  - "Large sample patches" (over 64,000,000 bytes in sample capacity, referred to as 'pink patches') are not automatically applied (unknown capacity is treated as normal). After selection, `Space` opens a confirmation overlay for instrument, patch, and capacity; `Enter` within it starts preparation/application. `Esc` during confirmation cancels without changing. Once applied, re-confirmation is needed. A progress overlay is displayed during loading; cancellation during this is not possible. `Space`/`Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is same as `OK`).
  - `Random` or the `R` key within the browser selects uniformly from the displayed results. Shortcuts `T`/`R`/`M`/`/`/`h`/`j`/`k`/`l`/numbers/`Enter` do not work while typing, and `R`/`M` do not work while overlays or save menu are displayed within the browser. For "pink patches", only selection occurs; the same Space → Enter confirmation is needed. `Esc`, `Enter`, or the OS close button for the browser window only closes the browser, preserving the main app and the last successfully applied patch, discarding uninitiated requests and old preparation results. Already started processes will complete. `Esc`/clicking outside overlay when confirmation overlay/Solo/Mute/save menu are open only closes the child, and does not pass the same input to the browser or background operations. OS close closes the entire browser. It can be reopened, and there's no undo operation to revert to the original patch.
  - Search conditions can be added to Role via `Save condition to Role` in the hamburger `☰` menu, and deleted via `Delete` in Preset. Adding from ALL goes to Etc. They are handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` in cat's config directory. If loading is corrupted, the original file is protected, writing is stopped, and save failure is displayed on screen. Role/Preset/Search/Solo/Mute are retained during the same app session; only saved user-defined conditions are restored after restart. TUI conditions and Favorites are not shared/migrated.
  - During `Browse patches` preparation, current playback continues, and conflicting load/scan/patch operations are disabled. There may be waits for sample loading for Floe, etc. MML/Velocity/CC1 global selection, effect order/state/Bypass are maintained, and after success, the state for next launch is saved. Favorites/History are not automatically added. Application success and save failure are displayed separately. Progress of preparation/application is updated even when child windows are in focus.
  - Patch browser candidates are CLAP patches from Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, Vaporizer2 in the existing cmrt catalog, matching the installed instrument by format + ID.
  - The patch browser catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode. VST3 fallback and unknown instruments are excluded. Effect preset selection uses Browse effects in the effect routing section. Dexed retains program specification within cartridges. Floe/sforzando external samples are needed in their original locations; there is no function to duplicate/save samples.
  - In environments without cmrt and where the above `catalog.json` does not exist, Surge XT's patch list is obtained on each launch from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns), saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json`, and used as candidates for Browse patches. If it cannot be obtained, the previously saved list is used; if that's also unavailable, patch browser displays a loading failure. A loading wait message appears until retrieval is complete (up to 10 seconds). The list is generated from Surge XT 1.3.4; Surge XT versions with different content will not show added patches, and selecting a non-existent patch will display preparation failure. It does not switch if `catalog.json` exists but cannot be read. Reasons are in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - If preparation/application fails, the reason is displayed. The original configuration, state, and playback status are restored, and instances where state restoration also failed are suppressed from saving. Save failure after patch application is displayed as applied. Instrument-specific preparation/application conditions and verification scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - You can save multiple patches/settings as favorites using `★ Add favorite` for each plugin. For instruments, playback patterns, stopped selection, and confirmed MML/chord text are also saved. For effects, playback settings are not saved. Names are automatically generated and can be changed later.
  - Loading a different plugin automatically records the replacing plugin's patch, settings, and playback pattern to History before the switch. The name is `Plugin Name Sequence Number`. Re-loading the same plugin or restoration on startup does not add to History. Confirmed MML/chord text and stopped selection for instruments are recorded within the same scope as favorites. Playback settings are not recorded for effects. If saving fails, the switch is aborted.
  - During manual/automatic saving, it checks for duplicates of the same plugin, role, playback pattern, confirmed MML text, and state, reusing saved items and names. For History's automatic recording, differences only in stop state are treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - In [Plugin-specific processing policy/list (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md), you can check product-specific exceptions, reasons, scope, and limitations. For confirmed state format of TyrellN6 CLAP, the compressed part that changes only with playback is excluded from duplicate detection, and patches with the same text-based patch settings are grouped. Confirmed PCore `UI_op=9/10` and empty lines at the end of text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed format of Vaporizer2 CLAP, minute differences in MSEG time/coordinates arising during loading/saving (absolute difference of specified items: time <= 0.00011 ms, normalized coordinates <= 0.000001. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - Switch lists using the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order, with new additions placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON enables reordering with `↑` / `↓` at the end of rows, saving immediately. Row click loading and effect removal behave as before. `Sort by plugin name` sorts once by plugin name in ascending order and saves (case-insensitive, preserving relative order for same-name items). Manual adjustments are still possible afterward. Reordering is disabled during filtering, and reordering mode is OFF on startup. See [ADR 0017](docs/adr/0017-manual-favorites-order.md) for background and specifications.
  - History is sorted by the latest registration. If a patch is deemed identical, the existing patch is reused, its registration time is updated, and it moves to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are maintained. All old-format items are migrated to History, and all items not starting with `auto ` are also kept in Favorites (because old formats did not record the origin of automatic saves, ensuring all renamed automatic saves are included in History). See [ADR 0016](docs/adr/0016-history-instead-of-automatic-favorites.md) for policy and reasons.
  - History does not display Rename/Delete operation menus. Renaming/deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If CLAP plugins with the same name, manufacturer, and type exist, VST3 plugins are hidden from the Plugins list (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are displayed). Turning OFF shows both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on next launch. It does not change Favorites or the format loaded/restored.
  - For both tabs, clicking names/images loads them; for connected effects, clicking again disconnects them. The currently loaded/connected row is highlighted, and color is maintained even when effects are bypassed. `Load` / `Add` / `Remove` buttons in Plugins also perform these operations.
  - Favorites are displayed one item per line. Long names and supplementary info are truncated; hovering shows the full text. Waveform/spectrum are displayed below Routing on the right; the width of the right panel can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display between bottom-right / bottom of the screen (defaulting to bottom-right on startup).
  - For sforzando CLAP's confirmed state format, identical patches are not duplicated based solely on save counter and note/CC1 playback value differences; existing favorites are reused. Differences in patch settings are preserved. See [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` is displayed to the left of each row. Clicking a favorite name restores the patch/settings. For instrument favorites, the saved confirmed text and playback pattern selection are restored. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites in old format without text retain the current text; old-format stopped items maintain the current selection. If conversion, state loading, or connection fails, the original text/selection/playback state is restored, and the failed content is not saved to the session. Instrument favorites maintain connected effects; effect favorites maintain the instrument and MML. An instrument is required to call up an effect.
  - Connected effect favorites display `Connected - click again to remove`; re-clicking the name removes that effect. This also applies after editing settings or during Bypass. The instrument, other effects, and saved favorites remain. Another patch of the same format + ID switches settings at the current position, and unconnected effects are added at the end. The selected favorite for each effect is also restored on next launch.
  - Rename/Delete options are available via `…` at the end of the row. Edits after calling or traditional automatic saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Short audio dropouts may occur during saving/restoring, and the scope of what can be saved is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` for the oscilloscope is unchecked, spectrum, oscilloscope, and Lissajous appear side-by-side at the bottom of the screen.
  - A 45° rotated Lissajous displays the output of the last 50ms (up to 4096 samples), reflecting effects and Bypass. A common automatic gain keeps volume differences and aspect ratios; in-phase at the same level is a vertical line, out-of-phase is a horizontal line, and sine waves with phase differences are ellipses or circles. Left-only output is a diagonal line from top-left to bottom-right, right-only is top-right to bottom-left, mono is a vertical line, and silence is a central dot.
  - It updates independently of notes, cycle counts, or trigger settings, and `Show analysis labels` can display headings. Axis and drawing methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation of the last 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is out-of-phase. Negative values are shown in red, serving as a guide to judge cancellation when mixed to mono. It shows "—" for silence or if one side is silent.
- Waveform Display
  - `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) output, reflecting effects and Bypass, superimposed. Mono displays the same waveform for both.
  - The display width is determined from the MIDI note number of the last Note On (A4 = 440Hz), and `1`, `2`, `4`, `8 cycles` can be selected. After Note Off, the tail continues to be displayed based on the last note. Output containing chords or detuning may not repeat perfectly at that width.
  - `Zero cross` aligns to the negative → positive crossing of the left channel. If the left channel is silent, it uses the right channel; if no crossing, it displays from the beginning.
  - `Similarity` comprehensively searches for the position with the maximum correlation coefficient to the previous display, from the beginning of the buffer for the display width, one sample at a time. In case of a tie, the earlier position is chosen, and the same starting position is used for left and right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when a note is sounded, playback pattern, display cycle count, trigger method, or audio stream changes. A waiting display appears until sufficient data is collected.
  - Exploration is performed on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - Initial display settings are 4 cycles and zero cross. Display settings are not saved.

## Settings: Window Position/Size

You can specify the main window's startup position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved to `status.json` in the same directory; normal saving does not overwrite the TOML. If states are mixed in an old TOML, they are migrated on first launch, the original file is saved as `config.toml.before-status-migration.bak`, and then only state items are removed. Configuration comments, formatting, and unknown items are preserved, and independent comments attached to deleted items remain at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates and are used as the startup position if both are specified. If omitted, it defaults to the traditional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. Either can be specified alone; omitted values or values <= 0/non-finite revert to width 900, height 600. Changes are reflected on next launch. The plugin placement method is currently `right_then_bottom_right` only, and it behaves the same if omitted. GUI size and placement are calculated based on actual window frame, screen work area, and DPI. Automatic saving of manually moved main window or GUI positions, or per-plugin position specification, is not performed.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this app's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` are always referenced locally via Cargo.toml. play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` requires their re-export. The patch browser uses public Role/Preset/filter APIs and tui-core's sample capacity detection. Browse effects uses play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions, and tui-core's search conditions. Insufficient checkouts or missing APIs will result in build errors; no automatic switching to Git versions occurs.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. UAPMD's location during build is specified via an environment variable. `config.toml` is not read as a build setting.

Do not use `cargo install`; directly execute the generated exe in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require re-configuration/re-build. If `target/shim` was created with a different version of VS (e.g., VS 2022), it will continue to build with that version, so delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe and unpacked into a cache directory for loading when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency structure and operational policy. Local switching of TUI is not necessary for building with cat. For cross-repo builds within TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; 'off' is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the contents of each checkout. When verifying, record the HEAD and diffs of each repo.

Launch the GUI with no arguments. `--help` and `--version` are also available.

## For Developers

### Recording

When reproducing first-time installation for introduction videos, use `cargo build --release --features recording-mode`.
An empty save location is created under `%LOCALAPPDATA%\cat-plugin-player-recordings\` on each launch.
It does not read/write normal settings/history, only detecting Surge XT / Surge XT Effects CLAP.
It does not refer to the cmrt catalog, acquiring a new catalog for Surge.
The absence of Surge XT is not faked; it's operated under the assumption that a human or coding agent uninstalls it before recording.
After recording, revert to the normal exe with `cargo build --release`.
Refer to [ADR 0026](docs/adr/0026-recording-mode.md) for scope and limitations.

### Surge XT Effects state table

When Surge XT is updated, etc., run the following in an environment with Surge XT Effects installed to regenerate the table:

```
cargo run --release --example export_surge_fx_states -- ..\cat-music-patterns\surge-xt-fx-states.json
```

Committing and pushing to cat-music-patterns is done manually. Refer to [ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md) for format and reasons.

</details>

# Future Brainstorming
- ※Subject to change based on mood
- Expand on strengths not present in [cmrt](https://cat2151.github.io/clap-mml-render-tui/README.ja), such as VST3 and GUI support.
  - Integrate GUI edit results status with cmrt.
    - Load and utilize edited patches and effects in cmrt.
  - Experiment with egui for various features.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, playing multiple SMFs together, replacing patches and effects?
    - Should these be separated into different apps? Or are they better in this app due to patch/effect replacement? What kind of workflow is envisioned?
- Utilize various cmrt crates for different tasks.
  - ※This will also refine the crates, and we benefit from using them, so it's a win-win.
  - CLI import / export
    - The cmrt keyboard screen can play patches, effects, arpeggios, etc., with a single command from CLI. Enable the same (to the level where it works by simply changing `cmrt` to `cat-plugin-player`).
      - Frequent breaking changes are fine. Manual migration later is possible. Better than being paralyzed by fear of change and halting progress.
  - SMF import / export
    - Currently, recording audio plugin names and patch names to SMF for output is implemented; further develop this.

# Concept, What We Aim For
- Instrument
  - For example, a synthesizer with a MIDI keyboard makes sound when powered on and a key is pressed.
    - We aim for that level of ease of use.
      - Minimize operations until sound is produced.
- Sound Production
  - We aim to maintain "it sounds on the author's environment" as much as possible.
- Immediate Sound
  - From app launch to sound production, it takes a perceived 0.2 seconds. We've made playback as fast as possible.
    - Processes like GUI display preparation are done after the sound.
      - Sound playing before GUI display could be seen as a sound version of a splash screen.
    - You need to run the release-built exe directly (running via cargo introduces timestamp checks, and debug builds have debug code, both adding to startup latency).
- Educational Use
  - More oriented towards educational use than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

# Casual Thoughts Corner
- ※Related to the concept
- I love audio plugin preset sounds.
  - Because they are reproducible.
  - Because they are freely shared, everyone can benefit.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It's the synthesis of each layer, from audio plugins to the PC keyboard.
    - There's an intermediate layer.
      - That's the plugin host, the performance UI.
        - This is a layer where vibe coding can be applied.
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - Meaning, we target beginners, including even cats and babies (e.g., Android tablets).
  - ※This is merely a metaphor. Actual operation on Android tablets is unconfirmed. The likelihood of striving to make it work on Android if it doesn't is also low.

# Out of Scope, What We Don't Aim For
- Robustness
  - Absolutely no bugs, never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Convenience
  - Extreme pursuit of convenience and ease of use.
- Performance
  - High-speed operation with no CPU load in any environment. Audio never drops out.
- Requests
  - Immediate response to all user requests.

# License
- MIT