# cat-plugin-player

A lightweight app for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- Audio plugin sounds are fantastic!
- To play them, you often need to launch a DAW and perform preliminary setup.
- Lightweight apps for easy playback sometimes require account registration to obtain, or their user experience wasn't to my liking.
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding" (rapid prototyping).
- That's what we decided to do.

# Installation, Browsing Patches & Effects

If you haven't installed any audio plugins, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) like this. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- **Detection**
  - Upon launch, it automatically detects installed plugins.
- **Playback**
  - Pressing the `Load` button automatically starts playback after the plugin loads.
- **GUI**
  - You can display the plugin's GUI by pressing the `Show UI` button.
- **Patches**
  - To change patches, use the plugin's GUI.

※↑This section is human-written. AI is prohibited from writing here.

# Features (Detailed AI-generated additions. Needs revision for readability later)

The oscilloscope display method and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- **Detection**
  - If plugin detection succeeds but no instruments are found, a prompt to install the free Surge XT via winget is displayed. It's also displayed if only effects are present. The prompt is in Japanese if Windows' display language is Japanese, otherwise in English. Even if you choose `Later`, it will show every time the app launches as long as no instruments are found.
  - `View Terms of Use` opens the Surge XT license (GPLv3) in your browser. Pressing `Agree to Terms and Install` executes winget and displays progress as "1/4 Preparing", "2/4 Downloading", "3/4 Installing", "4/4 Loading" with elapsed seconds. If a Windows confirmation screen (User Account Control) appears at 3/4, please select "Yes". Upon success, it automatically rescans the plugin list, loads Surge XT (CLAP), and starts playback as if `Load` was pressed. In this case, the previous instrument and effect states are not restored. Even if you close the prompt, the installation continues, and upon completion, a rescan and load occur.
  - If winget is not present or installation fails, winget's message and a manual download link are displayed, and recorded in `error.log`. Only Surge XT is included, and download percentage is not shown. The reasons for this choice are in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- **GUI**
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, automatically takes a screenshot, and resizes it to a PNG that fits the list's display frame and screen zoom level while maintaining aspect ratio. It's saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`. Plugins with matching name and vendor (ignoring leading/trailing spaces and case) share the same image for VST3/CLAP. If name/vendor are insufficient, they are distinguished by format + ID.
  - Previously saved format-specific images are automatically migrated and used as shared images based on list/favorite identification. Patch states and favorite load destinations continue to be distinguished by format + ID.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while maintaining aspect ratio. Hovering over an image shows it at its saved size. Plugins with no GUI displayed show a temporary icon of the same height.
  - The GUI prioritizes placement to the right of the main window, aligned with its top edge, within the screen's working area. If it doesn't fit, it's placed in the bottom-right of the main window's screen working area. This placement occurs when displayed and when the GUI is resized; it does not constantly follow the main window's movement. The GUI is not scaled down; if its width is greater than the screen, its left edge is aligned with the working area; if its height is greater, its top edge is aligned.
  - If an empty image or a failed capture occurs, retries happen at 1-second intervals for up to 20 seconds from GUI display. Saved empty images are treated as uncaptured, and opening the GUI will replace them with a valid image. After success, no re-capturing occurs.
  - If capturing fails even after retries, an error is displayed. Opening the GUI again will reattempt. GUIs that do not support capturing will remain with a temporary icon. If you wish to re-capture or if the saved PNG is corrupted, delete the relevant shared PNG and any remaining old format-specific PNGs, restart the app, and open the GUI. (Old PNGs are kept during migration).
- **Playback**
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on the next launch. Stopped state, selected playback pattern while stopped, and confirmed MML/chord text are also restored. Unconfirmed edits are not saved.
  - CLAP Note On/Off messages are sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of the patch and validation records are documented in [CLAP Note format explanation](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or a dropdown. The arrows cycle through types at both ends and do not include "stop". Switching during playback releases all notes in the current phrase and starts the new phrase from the beginning. While stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop. In the main window, the `Space` key also toggles start/stop (except when typing text). The selected MML/chord progression playback pattern and stopped state are restored on the next launch.
  - To preview MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover for shortcuts and input methods). Example single note: `t120 c8r8d4e4r4`. Example chord progression: `C F G C`. Both are accepted in the same input field; chord notation conversion is attempted first. Capital `C` becomes a chord, lowercase `ceg` becomes MML.
  - Input is confirmed with `Enter` / `Confirm` and canceled with `Esc` / `Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are used for text input, and `Space` for play/stop does not work.
  - Confirmed phrases loop playback with `Play` / `Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses normal overall playback. If playback was active just before opening, after confirmation, the new phrase plays from the beginning; after cancellation, the original playback pattern resumes from the beginning. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to normal playback patterns. The text and selection/playback state are saved immediately after confirmation; the restored playback state is also saved upon cancellation. These are restored on the next launch or from instrument favorites/History. Unconfirmed text is not saved, and cancellation or conversion failure retains the last confirmed text.
  - Input changes or cursor movement cause a single note/chord at that position to be auditioned once. This also applies to mid-edit or paste operations. Moving within the same articulation unit or merely redrawing does not re-audition. Previous tempo, octave, and note length are reflected; no sound is produced at rest or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the audition sound ends. The audition sound is released upon the next articulation unit, cancellation, confirmation, or instrument change. Auditioning does not alter the confirmed text or normal selection.
  - For confirmed MML loop playback, the `Velocity` field's setting takes precedence over any velocity specified within the input. CC1 can also be controlled from the `CC1 modulation` field. If the event capacity for a very dense phrase is reached, currently sounding notes are released, and playback stops. You can restart with `Stop` → `Play`.
  - Patch changes via app Favorites/History, Browse patches/Random patch/Random effect/Browse effects maintain the stop/play state. During playback, the phrase plays from the beginning, not continuing any mid-phrase waits from before the change. Leading rests specified in MML are preserved. While the patch browser or effect browser is open, `Space` for play/stop and `i` for MML input are suppressed. Patch changes made through a plugin's custom GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is reconstructed due to loading, state restoration, or connection changes, it waits for the first phrase to start while continuing normal audio processing, to preserve the initial sound. At 48kHz, the wait is approx. 21ms for Six Sines and approx. 85ms for TyrellN6. MML's leading rests are added on top of this. See [ADR 0019](docs/adr/0019-random-patch-all-catalog.md) for application conditions and validation records.
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` using left/right arrows or a dropdown. The initial value is 100. Range selection increases values in Note On order within a phrase and decreases them in the next loop, repeating this action. Changes are reflected from the next note without restarting the phrase and are restored on the next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or a dropdown. The initial value is 0. `sweep` repeatedly changes from 0 to 127 in 2 seconds, then from 127 to 0 in the next 2 seconds. The selection is restored on the next launch.
  - Guitar open strings send MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) as Note On in sequence at 125ms intervals. Two seconds after the last Note On, all six notes are simultaneously Note Off, repeating after 0.5 seconds.
  - Memory of the `Sequence` type selection, start, and stop occurs on successful `Load`. Memory persists after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - Upon exit and `Remove`, the binary state of patches/parameters exposed by plugins via UAPMD's state API is saved to `%LOCALAPPDATA%\cat-plugin-player\states\` per plugin. It is restored before playback starts on the next launch or when the same plugin is reloaded.
  - `Load` for an instrument replaces only the instrument, while maintaining connected effects and settings. The state of the replaced plugin is saved, and switching occurs only after the new plugin is successfully generated, restored, and connected. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only obtains component state. This does not include duplication of external samples or saving upon forced termination.
  - On launch, the previous instrument and effects are loaded directly, states are restored, and playback begins. The GUI is then initialized, and the list is scanned in the background.
  - If direct loading fails due to old history or file relocation, restoration occurs from the scan results after GUI display. Old history items are updated once played.
  - If the main instrument is CLAP Floe, when audio processing is reconstructed due to loading or patch restoration, the start of the first phrase is delayed by 100ms. MML's leading rests are added further. This is a provisional measure for observing a symptom of initial sounds disappearing, and resolution on actual hardware is unconfirmed. See [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- **Effect Routing**
  - Serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. Connected effects can be removed one by one with `Remove`. Duplicate connections of effects with the same format + ID are not allowed.
  - `Random effect` next to `Sequence` draws a preset for an installed CLAP effect from the same catalog and selection targets as cmrt's effect selector. If there are 0 effects, it adds the first one; if 1, it replaces that effect; if multiple, it replaces one of the connected effects uniformly. Repeated presses do not increase the number of effects and maintain the order, settings, and Bypass state of other effects. To avoid duplicate connections, format + IDs already connected (excluding the target for replacement) are excluded from candidates. If no suitable candidates are available for the target slot, a reason is displayed, and no change is made.
  - Random effect requires an instrument. Candidates are obtained from the default effect/preset locations scanned by the play-server shared core; no VST3 fallback is performed. During preparation, current playback continues, and load/scan/patch operations are disabled. While stopped, it remains stopped; while playing, it restarts the current phrase from the beginning, maintaining global MML/Velocity/CC1 selections. On failure of generation, application, or connection, it reverts to the original configuration, without automatically drawing another candidate. After success, the applied state and connection are saved for the next launch; Favorites/History are not added. Save failures are displayed as "applied".
  - `Browse effects` below `Random effect` or the `E` / `X` keys open an independent maximized effect browser window where you can select presets from a list. It cannot be opened while editing MML. Filter by Category → Kind on the left and by search. Classification and selection targets are the same as cmrt's effect selector. Search is case-insensitive regex, space-AND, `-condition` exclusion, applying to preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regex during input displays an error and retains the previous list. The top row has Search, Target frame, and `Random`. `/` within the browser moves input to the search field, and `R` picks a random one from the filtered results. Multi-column list display, movement with arrow keys / `h` / `j` / `k` / `l` / number prefixes / `PageUp` / `PageDown`, and closing with `Enter` / `Esc` are the same as Browse patches. In results with mixed effects, effect names are added per effect, and if Kinds differ, preset names are color-coded per Kind.
  - The target frame is chosen from `Add new effect` or `Replace n` of a connected effect. When opened, it defaults to the last frame written by this browser, otherwise the last effect, or `Add new effect` if no effects. Applying with `Add new effect` makes the added effect the target, and subsequent selections replace that frame, so the number of effects does not increase by continually selecting. If the target frame is removed, the last effect becomes the target. Presets of effects connected to frames other than the target are grayed out, and selecting them displays a reason and prevents change. Presets added to the target frame by this browser are marked with `▶`, and opening the browser selects them (no re-application occurs).
  - Clicking/key-navigating or filtering to another candidate immediately requests application. Simply opening does not change the effect. You can select the next even during preparation; unstarted selections are replaced by the latest one and canceled upon closing. An instrument is required, and preparation/playback continuation, failure recovery, and saving for next launch are the same as Random effect. Category, Kind, search string, list selection position, and target frame are saved upon exit and restored on the next launch. Key navigation starts from the restored selection. Category/Kind are restored by name; if gone, it reverts to `ALL` in that field. Selection is restored by preset; if it's no longer in the catalog, it becomes unselected, and key navigation starts from the top. The target frame is restored by its position in the chain; if no effect exists at that position, it defaults to the frame at opening.
  - `Routing` displays the connection order. You can reorder effects by dragging the `↕` button up/down. While dragging, the moving effect name and insertion line are shown. Instruments are not reorderable. `Show UI` for each plugin can edit patches or effects. `Instrument` / `Effect` are displayed with the same type color as the left-side list.
  - Removing all effects reverts to direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing passthrough while retaining instance and settings. It can also be toggled with the `b` key (does not work in MML input screen, Browse patches/Browse effects, or while typing text). While bypassed, connected effects in Routing are grayed out and `Bypassed` is appended.
  - `Sequence` is for instruments. Stopping `Sequence` continues audio processing to allow effect tails.
  - On next launch, instrument, order of all effects, their respective states, and Bypass state are restored before playback. History of one effect in old format can also be loaded. If effect restoration fails, an error is displayed, it reverts to direct instrument connection, and the saved configuration is retained.
  - Additions, removals, and reordering are saved only if the new connection setup succeeds. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication, stereo → mono is left/right average. Auxiliary inputs are silent.
  - Connection or Bypass changes may cause a short audio dropout, interrupting sounding notes and tails. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- **Patches**
  - `Browse patches` near `Random patch` or the `T` key opens an independent maximized patch browser window on the same screen as the main window. It does not change the main window's position or size. It cannot be opened while editing MML. Filter by Role → Preset → search with frames; the left condition area and right Patches list are scrollable. It uses remaining width and height for the list, allowing operation even with unmaximized or narrow windows. The top row has Search, `Solo/Mute`, `Random`, `☰`. The search field uses remaining width, and the `/` key within the browser moves input to the search field. Details of instrument conditions can be checked by hovering over `Solo/Mute`. Clicking this button or the `M` key within the browser opens a vertical Solo/Mute overlay; `OK` or `Enter` confirms all changes. Solo/Mute appear right next to the instrument name. Clicking the selected mode again reverts to normal; `Esc` / clicking outside the overlay discards changes. Multiple Solo is treated as OR, Mute as exclusion. Search uses case-insensitive regex, space-AND, `-condition` exclusion, and `plugin:` condition (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases of integrated patches are also searchable. Invalid regex results in an error, stopping application,抽選 (random selection), and condition saving. If 0 items, review conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 items. Role, Preset, and search string are saved on exit and restored on next launch. Preset is restored by name; if the condition is deleted, it reverts to `ALL` for that Role.
  - The list displays patch name cells in single lines, wrapping into multiple columns (left → right → next row) based on width. In a narrow window, it reverts to a single column. Resizing maintains the selected patch; only changing column count does not reload. Clicking a patch does not move the list's display position. Arrow keys or `h` / `j` / `k` / `l` move according to the visual layout. Left/right move to adjacent patches (or ends of adjacent rows at row ends); up/down move to rows above/below, moving to the end of a short final row if no patch is directly below. Entering a number first, like `2j` or `10k`, moves that many times (stops at ends; numbers are canceled by non-movement keys). `PageUp` / `PageDown` move 10 times up/down, and combining with numbers multiplies the movement. Key navigation, Random, filtering, and column count changes only scroll to make the selected row visible if it's off-screen. If all patches fit, no scrolling occurs. For single-instrument results, the instrument name is omitted; for mixed results, instrument names are retained. Text color shows only one difference in results. If instruments are mixed, the instrument name part is color-coded per instrument. For a single instrument, if Preset is `ALL`, patch names are color-coded per relevant Preset; if that doesn't distinguish, then per category. Colors are Monokai-based. If all are the same, normal text color is used. Colors cycle through 5, so distant groups might have the same color. For "pink" patches, "pink" is prioritized in the patch name. Classification folders are omitted; if distinguishing same-named patches, necessary paths are retained. Dexed program specification is maintained. Hovering shows the full display path, source path, valid category, and merge count (if 2 or more). Load/Size are not permanently displayed in the list; `no category` / `1 merged` are also not shown. Normal patches immediately request application when selection changes via mouse/key navigation; the same applies when a different candidate is chosen by condition change. Simply opening does not change the patch. When opened, if the currently playing patch (applied via Browse patches/Random patch) is in the current results, that patch is selected and scrolled into view (no re-application). The playing patch is saved on exit and inherited as the restored patch on next launch. Adding/removing/Random effect maintains this inheritance. If the playing patch is not in the results or unknown, the selection from when it was last closed is retained (unselected when opened for the first time after launch). After changing patches via Favorites/History, instrument load/delete, the playing patch is unknown, and selection is cleared. Key navigation starts from the selection when opened; if unselected, it moves to the top. The next selection can be made even during preparation; unstarted requests are replaced by the latest one. On normal successful application, duplicate patch names or fixed whitespace at the bottom are not shown. A short message is displayed only if the selected and applied patches differ, indicating that the previous patch might continue playing for unconfirmed "pink" patches. The "pink" Space prompt and application failure/save errors are displayed.
  - "Pink" patches with sample capacity of 64,000,000 bytes or more are not automatically applied (unknown capacity is treated as normal). After selecting, `Space` opens a confirmation overlay for instrument, patch, and capacity; pressing `Enter` within it starts preparation/application. `Esc` during confirmation cancels without changing. Once applied, re-confirmation is required. A progress overlay is shown during loading, and cancellation is not possible mid-way. `Space` / `Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is same as `OK`).
  - `Random` or the `R` key within the browser picks uniformly from the displayed results. While typing text, `T` / `R` / `M` / `/` / `h` / `j` / `k` / `l` / numbers / `Enter` shortcuts do not work, and `R` / `M` do not work while overlays or the save menu are open in the browser. For "pink" patches, only selection is made, requiring the same Space → Enter confirmation. The external Random patch continues to pick uniformly from all available patches as before. `Esc`, `Enter`, or OS close of the browser window closes only the browser, preserving the main app and the last successfully applied patch, discarding unstarted requests and old preparation results. Already started processes are completed. `Esc` / clicking outside the overlay when the confirmation overlay, Solo/Mute, or save menu is open closes only the child, not passing the same input to the browser or underlying operations. OS close closes the entire browser. It can be re-opened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to Role via `Save condition to Role` in the hamburger `☰` menu, and deleted via `Delete` in Preset. Adding from ALL goes to Etc. They are handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` in the `cat` config directory. If loading is corrupted, the original file is protected, writing is stopped, and save failures are displayed on screen. Role / Preset / search / Solo / Mute are retained during the same app launch; only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared or migrated.
  - Browse patches uses the same catalog, supported CLAP instruments, state application, and failure recovery as Random patch. Global MML/Velocity/CC1 selections, effect order/state/Bypass are maintained, and after success, the state for the next launch is saved. Favorites/History are not automatically added. Application success and save failure are displayed distinctly. Progress of preparation/application updates even when child windows are in focus.
  - `Random patch` in `Sequence` uniformly draws from all available CLAP patches for Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, and Vaporizer2 found in the existing cmrt catalog, matching installed instruments by format + ID. It's not a uniform draw per instrument; Dexed, having more candidates, will be chosen more frequently. The tooltip displays target instruments and total candidates. The button is not displayed if no corresponding candidates exist.
  - Random patch's catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` read-only. Random patch does not substitute VST3, draw effect patches, or target unknown instruments. For effect preset selection, use Random effect / Browse effects in the effect routing section. Dexed retains program specifications within cartridges. Floe/sforzando external samples are required in their original locations; there is no function to duplicate or save samples.
  - In environments without cmrt and thus without the `catalog.json`, Surge XT's patch list is fetched from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) on each launch, saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json`, and used as candidates for Browse patches/Random patch. If fetching fails, the previously saved list is used; if that's also unavailable, Random patch is not displayed. A loading message appears until fetching completes (max 10 seconds). The list was created from Surge XT 1.3.4; Surge XT versions with more patches will not show the new ones, and selecting a non-existent patch will show a preparation failure. If `catalog.json` exists but cannot be read, it does not switch. Reasons are in [ADR 0025](docs/adr/0001-scope-follows-waveform-position.md).
  - During Random patch preparation, current playback continues, and conflicting load/scan/patch operations are disabled. There might be a wait for sample loading for Floe, etc. While stopped, it remains stopped after application; while playing, it restarts the current phrase from the beginning. Global MML/Velocity/CC1 selections and effect order/state/Bypass are maintained. After success, the applied state is saved for the next launch; Random patch itself does not add to Favorites/History.
  - If preparation/application fails, a reason is displayed, and another patch is not automatically drawn. The original configuration, state, and playback state are restored, and instances where state restoration also failed are prevented from saving. Save failures after patch application are displayed as "applied". Instrument-specific preparation/reflection conditions and validation scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - Using `★ Add favorite` for each plugin, you can save multiple instances of its current patch/settings as favorites. For instruments, playback patterns, selected patterns while stopped, and confirmed MML/chord text are also saved. For effects, playback settings are not saved. Names are automatically generated and can be changed later.
  - When another plugin is `Load`ed, the patch/settings/playback pattern of the plugin being replaced are automatically recorded to History before the switch. The name is `Plugin Name Sequence Number`. It is not added on re-loading the same plugin or during startup restoration. Confirmed MML/chord text and selected patterns while stopped for instruments are also recorded, within the same scope as favorites. Playback settings are not recorded for effects. If saving fails, the switch is aborted.
  - During manual/automatic saving, duplicate plugins, roles, playback patterns, confirmed MML text, and states are checked, and existing items and names are reused. In automatic History recording, only differences in stop state are treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - [Plugin-specific processing policy/list (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides exceptions, reasons, scope, and limitations for each product. For TyrellN6 CLAP's confirmed state format, compressed parts that change only with playback are excluded from duplicate judgment, grouping patches with identical text patch settings. Confirmed PCore `UI_op=9/10` and trailing blank lines in text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For Vaporizer2 CLAP's confirmed format, minute differences in MSEG time/coordinates arising from load/save (absolute difference for specified items: time less than `0.00011` ms, normalized coordinates less than `0.000001`. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - Switch lists with the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order; newly added items are placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning on `Reorder favorites` allows sorting with `↑` / `↓` at the end of the line and saving immediately. Loading by clicking a line and removing effects work as before. `Sort by plugin name` sorts once by plugin name ascending (case-insensitive, same-named items maintain relative order) and saves. Manual adjustments are still possible afterward. Sorting is disabled while filtering; sort mode is OFF on launch. See [Adoption history and specifications (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is in order of most recent registration. If the same patch is detected, the saved patch is reused, its registration time is updated, and it moves to the top. Elapsed time is shown in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are maintained. All old-format items are migrated to History, and all items not starting with `auto ` are also kept in Favorites (because old formats did not record the origin of auto-saves, this ensures all renamed auto-saves are included in History). See [Policy and reasons (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus. Renaming/deletion is done in Favorites.
  - The global setting `☰` includes `Show only CLAP for duplicate plugins`, which is ON by default. If a CLAP plugin with the same name, vendor, and type exists, VST3s are hidden from the Plugins list (ignoring leading/trailing spaces and case differences in name/vendor; if name or vendor is empty, both are shown). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on next launch. Favorites and load/restore formats are not changed.
  - On both tabs, clicking the name/image loads; for connected effects, clicking again removes them. Currently loaded/connected rows are highlighted, and the color is maintained even when effects are bypassed. `Load` / `Add` / `Remove` buttons in Plugins perform the same operations.
  - Favorites are displayed one item per line. Long names/supplementary info are truncated; hover to see full text. Waveform/spectrum are displayed below Routing on the right; the right-side width can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display to bottom-right / screen bottom (bottom-right on launch).
  - For sforzando CLAP's confirmed state format, existing favorites are reused without creating new ones if only the save counter and playback values for notes/CC1 differ, thus preventing duplicates. Differences in patch settings are preserved. See [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for application scope and limitations.
  - Colored `Instrument` / `Effect` is displayed to the left of each row. Clicking a favorite name restores its patch/settings. For instrument favorites, the saved confirmed text and playback pattern selection are restored. While stopped, it remains stopped; `Play` plays the restored phrase. While playing, it switches to the saved pattern/MML. Old-format favorites without text retain the current text; old-format stopped items maintain the current selection. If conversion or state load/connection fails, the original text/selection/playback state are restored, and failed content is not saved to the session. Instrument favorites retain connected effects; effect favorites retain the instrument and MML. An instrument is required to call up an effect.
  - Connected effect favorites display `Connected - click again to remove`; re-clicking the name removes that effect. This works the same even after editing settings or during Bypass. The instrument, other effects, and saved favorites remain. Another patch with the same format + ID switches settings at the current position, and unconnected effects are added to the end. The selected favorite for each effect is also restored on the next launch.
  - Rename/Delete operations are available from `…` at the end of the line. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Short audio dropouts may occur during save/restore; the savable range is the same as traditional state saving.
- **Lissajous Display**
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` for the oscilloscope is unchecked, spectrum, oscilloscope, and Lissajous align horizontally at the bottom of the screen.
  - The Lissajous displays the most recent 50ms of output (max 4096 samples) reflecting effects/Bypass, rotated 45°. A common automatic gain keeps volume differences and aspect ratio; in-phase same-level signals are vertical lines, out-of-phase are horizontal, and sine waves with phase difference are ellipses or circles. Output only on the left is a diagonal from top-left to bottom-right; output only on the right is a diagonal from top-right to bottom-left; mono is a vertical line; silence is a central dot.
  - It updates independently of notes, cycle counts, or trigger settings. `Show analysis labels` displays headings. Axes and rendering methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation for the most recent 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is out-of-phase. Negative values are shown in red, serving as a guideline for judging cancellation when mixed to mono. It shows "—" for silence or if one side is silent.
- **Waveform Display**
  - `Oscilloscope` at the bottom of the screen displays L (green) and R (blue) outputs, reflecting effects/Bypass, overlaid. Mono displays the same waveform for both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and you can choose `1`, `2`, `4`, `8 cycles`. After Note Off, the tail continues to display based on the last note. Output containing chords or detuning may not repeat perfectly at that width.
  - `Zero cross` aligns to the negative → positive crossing of the left channel. If the left channel is silent, the right channel is used; if no crossing, it displays from the beginning.
  - `Similarity` comprehensively searches from the buffer start for the display width, sample by sample, for the position with maximum correlation to the previous display. Ties prefer the earlier position; the same start position is used for left and right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when notes are played, playback patterns change, display cycle count changes, trigger method changes, or the audio stream changes. A waiting display appears until sufficient data is gathered.
  - Exploration is performed on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - The initial display setting on launch is 4 cycles, zero cross. Display settings are not saved.

# Build & Test Procedures
- ※Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for instructions. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI how to install it, and it will be easy! The overall image is "easy with AI".
  - Building the C++ parts (UAPMD and shim) requires Visual Studio 2026 Build Tools (C++ workload, MSVC v145). Other versions like VS 2022 are not verified. Example installation: `winget install --id Microsoft.VisualStudio.BuildTools --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
- ※Next prerequisite: If Rust is not installed, ask AI how to install it, and it will be easy!
- As mentioned above, confirm that `../uapmd` has been built.
- Confirm that an audio plugin (instrument) like Surge XT is installed. For example, confirm Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the screen opens.
- Confirm that Surge XT, etc., are listed on the screen.
- Click the `Load` button.
- Confirm that it loads and sound plays.

# Miscellaneous

The AI-generated text is hard to read. I plan to fix it later.

## Window Position & Size Settings

You can specify the main window's launch position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved to `status.json` in the same directory, and normal saves do not overwrite TOML. If states are mixed in the old TOML, they are migrated on first launch, the original file is saved as `config.toml.before-status-migration.bak`, and only state items are removed. Settings comments, formatting, and unknown items are preserved, and independent comments attached to deleted items remain at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates, used as launch position if both are specified. If omitted, it uses the traditional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. You can specify only one; if omitted or if values are 0 or less/non-finite, they revert to width 900, height 600. Changes are reflected on the next launch. The plugin placement method is currently `right_then_bottom_right` only, and the default behavior is the same if omitted. GUI size and placement are calculated based on actual window frame, screen working area, and DPI. Automatic saving of positions moved manually for main window or GUI, and per-plugin position specification are not performed.

## Local Build and Run

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this app's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` are always referenced locally from Cargo.toml. Play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to the traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` needs their re-export. The patch browser uses public Role/Preset/filter APIs and tui-core's sample capacity determination. Random effect and Browse effects use play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions; Browse effects' search uses tui-core's search conditions. Missing checkouts or APIs will result in build errors; no automatic switching to Git versions is performed.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. UAPMD's location during build is specified via environment variable. `config.toml` is not read as build settings.

Do not use `cargo install`; directly run the exe generated in the default `target/release`. Building uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require reconstruction/rebuild. If `target/shim` was created with VS 2022 or another version, it will continue to build with that version, so delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe, extracted to the cache directory, and loaded when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency configuration and operational policy. Local switching of TUI is not necessary for building with `cat`. If building TUI across repositories, use TUI's existing `python scripts/cross_repo_local.py on` procedure; `off` is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the contents of each checkout. When verifying, record the HEAD and diffs of each repo.

Launch the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- ※Subject to change based on mood
- Enhance the strength of supporting VST3 and GUI, which cmrt lacks.
  - Automatic GUI operation?
    - Log GUI operations and reproduce them rapidly in cmrt TUI? Provide hints for plugin-specific customization?
  - Play around with egui
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates.
  - ※This will also refine the crates and provide benefits through their use, a win-win.
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can play a patch, effect, arpeggio, etc., with a single command from CLI. Enable the same (to the level where it plays just by changing cmrt to cat-plugin-player).
      - Frequent breaking changes are acceptable. Manual migration is possible later. Better than being stuck and hindering progress due to fear of change.

# Concept, Goals
- **Musical Instrument**
  - For example, a synthesizer with a MIDI keyboard makes sound when you power it on and press a key.
    - We aim for that level of ease of use.
      - Minimize the operations required to produce sound.
- **Producing Sound**
  - We aim to maintain "it sounds on the author's environment" as much as possible.
- **Immediate Sound**
  - From app launch to sound production: a perceived 0.2 seconds. Playback is prioritized to be as fast as possible.
    - Processes like GUI display preparation are performed after sound.
      - Sound playing before GUI display might be like a sound version of a splash screen.
    - Requires running the release-built exe directly (via `cargo` involves timestamp checks, and debug builds have debug code, both adding to launch time).
- **Educational Use**
  - It is geared more towards educational use than commercial use.
- **Experimentation, Exploration, Personal Use**
  - Frequent breaking changes will occur.

# Random Thoughts Corner
- ※Related to the concept
- I love audio plugin preset sounds.
  - Because they are reproducible.
  - Thanks to being freely shared, everyone benefits.
- I want to provide an instrument UX.
  - Not a DAW UX, but an instrument UX.
- What is an instrument?
  - It's a combination of layers, from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host, the playback UI.
        - This is a layer where we can rapidly prototype to improve the UX.
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - Meaning the target audience is broadened to include beginners, even to the extent of cats and babies (e.g., an Android tablet).
  - ※This is purely a metaphor. Actual operation on Android tablets is unconfirmed. The likelihood of effort to make it work on Android if it doesn't is low.

# Out of Scope, Not Goals
- **Robustness**
  - Absolutely no bugs, never crashing.
  - Full compatibility. Loads and operates with all past settings and data.
- **Features**
  - Equipped with every possible feature. Usable for all purposes.
- **Convenience**
  - Extreme pursuit of convenience and ease of use.
- **Performance**
  - High-speed operation with no CPU load in any environment. Audio never glitches.
- **Requests**
  - Immediate response to all user requests.

# License
- MIT