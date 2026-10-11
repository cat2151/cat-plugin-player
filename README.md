# cat-plugin-player

A lightweight app for easily playing audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- Audio plugin sounds are fantastic!
- To play them, you often need to launch a DAW and perform setup beforehand.
- Lightweight apps for easy playback sometimes require account registration to obtain, or their operation felt "not for me."
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So, that's what I decided to do.

# Download

[Download Windows version](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

# Demo Video: Automatic Audio Plugin Installation, Browsing Patches & Effects

If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) for you, as shown here. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) for you, as described above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Click a plugin to load it and automatically start playback.
  - You can play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - Press the `Show UI` button to display the plugin's GUI and edit patches and effects.
- Patches
  - Each plugin has a variety of preset patches. You can easily select them using a unique patch browser (supported by some plugins only).
  - For plugins not supported by the patch browser, you can select patches directly from the plugin's GUI.
- Effects
  - Each plugin has a variety of preset effects. You can easily select them using a unique effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects directly from the plugin's GUI.

※↑ This section is written by a human. AI is prohibited from writing here.

# Features (Detailed additions by AI. This is hard to read and will be revised later.)

The oscilloscope display method and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Detection
  - If plugin detection succeeds but no instruments are found, a prompt to install the free Surge XT via winget is displayed. It also appears if only effects are present. The prompt is in Japanese if Windows' display language is Japanese, otherwise in English. Even if you choose `Later`, it will be displayed on every launch as long as no instruments are present.
  - `View Terms of Service` opens Surge XT's terms (GPLv3) in your browser. Pressing `Agree to Terms and Install` executes winget, showing stages like "1/4 Preparing", "2/4 Downloading", "3/4 Installing", "4/4 Loading" and elapsed time. If a Windows confirmation screen (User Account Control) appears at 3/4, choose "Yes". Upon success, the plugin list is automatically rescanned, Surge XT (CLAP) is loaded, and playback starts as with `Load`. In this case, previous instrument/effect restoration is not performed. Even if the prompt is closed, installation continues, followed by rescan and loading upon completion.
  - If winget is missing or installation fails, winget's message and manual download link are displayed, and logged to `error.log`. Only Surge XT is included, and download progress is not shown. The reason for this choice is in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- GUI
  - The first time the GUI is opened, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG with preserved aspect ratio, fitting the list display frame and screen zoom level. It's saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching name + vendor (ignoring leading/trailing spaces, case) share the same image across VST3 / CLAP. If name/vendor are incomplete, they are distinguished by format + ID.
  - Previously saved format-specific images are automatically migrated to shared images using the list/favorite identification info. Patch states and favorite load destinations continue to be distinguished by format + ID.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height, preserving aspect ratio. Hovering over an image shows it at its saved size. Plugins with no GUI displayed show a temporary icon of the same height.
  - The GUI prioritizes placement to the right of the main window, aligned to the top, within the screen's work area. If it doesn't fit, it's placed at the bottom right of the work area of the screen where the main window is located. It's positioned on display and GUI resizing, but does not constantly follow main window movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the work area; if its height is greater, its top edge aligns.
  - For blank images or failed captures, retries occur at 1-second intervals, up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured, and opening the GUI replaces them with a valid image. No re-capturing occurs after success.
  - If capture fails even after retries, an error is displayed. Opening the GUI again initiates a retry. Non-capturable GUIs remain with a temporary icon. To re-capture or if a saved PNG is corrupt, delete the relevant plugin's shared PNG and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are kept during migration).
- Playback
  - The last played instrument, connected effects, Bypass status, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next launch. Stopped status, selected stopped playback pattern, and confirmed MML/chord text are also restored. Unconfirmed edits are not saved.
  - CLAP Note On/Off messages are sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are detailed in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or a dropdown. Arrows cycle through the ends of types; 'Stop' is not included. Switching during playback releases all sounding notes of the current phrase and starts the new phrase from the beginning. When stopped, selecting a type does not start playback; `Play` starts it, `Stop` stops it. The main window's `Space` key also toggles start/stop (except when typing). The selected playback pattern (including MML/chords) and stopped state are restored on next launch.
  - To audition MML/chord progressions, open the input screen with the `i` key or `Sequence`'s `Chord` button (hover to see shortcuts and input methods). Example single note: `t120 c8r8d4e4r4`. Example chord progression: `C F G C`. Both are accepted in the same input field; chord notation conversion is tried first. Uppercase `C` creates a chord, lowercase `ceg` creates MML.
  - Input is confirmed with `Enter`/`Confirm` and canceled with `Esc`/`Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are for typing; `Space` for play/stop does not work.
  - Confirmed phrases loop with `Play`/`Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses general playback. If playback was active just before opening, it resumes from the new phrase after confirming, or the original pattern after canceling. If opened while stopped, it remains stopped after confirming/canceling, and `Play` starts general playback. You can also switch to regular playback patterns. The text and selection/playback state are saved immediately after confirmation, and the restored playback state is saved on cancellation. This is restored on next launch or from instrument favorites/History. Unconfirmed text is not saved; cancellation or conversion failure retains the last confirmed text.
  - Changing input or moving the cursor auditions the single note/chord at that position once. This includes in-place editing and pasting. Moving within the same pronunciation unit or merely redrawing does not re-audition. It reflects the previous tempo, octave, and note length, and does not sound at rest or command positions. General playback stops when editing is opened, and does not resume after the audition sound ends. The audition sound is released on the next pronunciation unit, cancellation, confirmation, or instrument change. Auditioning does not alter confirmed text or regular selection.
  - The velocity for looping MML playback prioritizes the `Velocity` field setting over any in-input specification. CC1 can also be controlled from the `CC1 modulation` field. If the playback event capacity is reached due to a very dense phrase, sounding notes are released, and playback stops. You can restart with `Stop` → `Play`.
  - When calling from app Favorites/History or changing patches via Browse patches/Random patch/Random effect/Browse effects, the stopped/playing state is maintained. If playing, the phrase starts from the beginning, without carrying over delays from the middle of the previous phrase. Leading rests specified in MML are preserved. While the patch/effect browser is open, `Space` for play/stop and starting MML input are suppressed. Patch changes made directly in the plugin's GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when reconstructing audio processing due to loading, state restoration, or connection changes, the first phrase starts after a delay to preserve the initial sound, while normal audio processing continues. At 48kHz, the delay is ~21ms for Six Sines and ~85ms for TyrellN6. Leading rests in MML are added to this. See [ADR 0019](docs/adr/0019-random-patch-all-catalog.md) for application conditions and validation.
  - In the `Velocity` field below playback patterns, you can select `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases values in Note On order within a phrase and decreases them in the next loop, repeating the behavior. Changes take effect from the next note without restarting the phrase and are restored on next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or a dropdown. The initial value is 0. `sweep` repeatedly changes from 0 to 127 in 2 seconds, then from 127 to 0 in the next 2 seconds. The selection is restored on next launch.
  - Guitar open strings: MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) are Note On'ed sequentially at 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Off'ed simultaneously, repeating after 0.5 seconds.
  - On successful `Load`, it is memorized when selecting sequence type, starting, or stopping. Memory persists after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, the plugin's binary state (patches, parameters, etc.) exposed by UAPMD's state API is saved to `%LOCALAPPDATA%\cat-plugin-player\states\` for each plugin. It is restored before playback starts on the next launch or when reloading the same plugin.
  - `Load` for an instrument replaces only the sound source, maintaining connected effects and settings. The state of the plugin being replaced is saved, and it switches after successfully creating, restoring, and connecting the new plugin. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. It does not include duplicating external samples or saving on forced termination.
  - On startup, it directly loads the previous instrument and effects, restores their states, then starts playback. After that, it initializes the GUI and scans the list in the background.
  - If direct loading fails due to old history or file movement, it restores from the scan results after GUI display. Old history is updated once played.
  - If the main instrument is CLAP Floe, when audio processing is reconstructed (e.g., during load or patch restoration), the start of the first phrase is delayed by 100ms. Leading rests in MML are added. This is a provisional measure to observe symptoms of missing initial sounds; resolution on actual hardware is unconfirmed. See [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - Serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. `Remove` a connected effect to remove only that one. Duplicate connection of effects with the same format + ID is not allowed.
  - `Random effect` next to `Sequence` randomly selects a preset from installed CLAP effects, using the same catalog and selection pool as cmrt's effect selector. If there are 0 effects, it adds the first one; if 1, it replaces that effect; if multiple, it replaces one of the connected effects equally. Repeated presses do not increase the number of effects and maintain the order, settings, and Bypass of other effects. To avoid duplicate connections, format + ID of already connected effects (other than the replacement target) are excluded from candidates. If no suitable candidate is available for the target slot, it displays a reason and does not change.
  - Random effect requires an instrument. Candidates are obtained from default effect/preset locations explored by the play-server shared core; no fallback to VST3. During preparation, current playback continues, and loading/scanning/patch operations are disabled. If stopped, it remains stopped after application; if playing, the current phrase restarts from the beginning, maintaining global MML/Velocity/CC1 selection. If generation/application/connection fails, it reverts to the original configuration, and no automatic re-selection of other candidates occurs. After success, the applied state and connection are saved for next launch; Favorites/History are not added. Saving failures are displayed as "applied".
  - `Browse effects` below `Random effect` or `E`/`X` keys open a maximized, independent effect browser window, allowing preset selection from a list. It cannot be opened while editing MML. Filter by Category → Kind on the left and by search. Classification and selection targets are the same as cmrt's effect selector. Search uses case-insensitive regular expressions, space for AND, `-condition` for exclusion, applying to preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regex during input displays an error and retains the previous list. The top line has Search, Target frame, `Random`. `/` within the browser moves input to the search field. `R` randomly selects from filtered results. Multiple column display, navigation with arrow keys/`h`/`j`/`k`/`l` and number prefixes, `PageUp`/`PageDown` for larger jumps, and `Enter`/`Esc` to close are the same as Browse patches. In results with mixed effects, effect names are added for each effect; if Kinds differ, preset names are color-coded by Kind.
  - The target frame is chosen from `Add new effect` or `Replace n` of a connected effect. When opened, it defaults to the last frame written by this browser, or the last effect if none, or `Add new effect` if no effects are present. Applying with `Add new effect` makes the added effect the target, and subsequent selections replace that frame, so the number of effects doesn't increase with continuous selection. If the target frame is removed, the last effect becomes the target. Presets for effects connected to non-target frames are grayed out; selecting them shows a reason and does not change. Presets applied via this browser to the target frame have `▶` and are selected when opened (not reapplied).
  - Clicking, key movement, or filtering that selects a different candidate immediately requests application. Simply opening does not change the effect. You can select the next even during preparation; unstarted selections are replaced by the latest one, and canceled upon closing. An instrument is required, and playback continuation during preparation, fallback on failure, and saving for next launch are the same as Random effect. Category, Kind, search string, list selection position, and target frame are saved on exit and restored on next launch. Key movement starts from the restored selection. Category/Kind are restored by name; if missing, they revert to `ALL` for that field. Selection is restored by preset; if missing from the catalog, it becomes unselected, and key movement starts from the beginning. The target frame is restored by its position in the chain; if no effect is at that position, it defaults to the frame chosen on opening.
  - `Routing` displays the connection order. Drag the effect's `↕` button up/down to reorder. While dragging, the moving effect name and insertion line are shown. Instruments cannot be reordered. `Show UI` for each plugin allows editing patches and effects. `Instrument`/`Effect` are displayed with the same type color as the left list.
  - Removing all effects reverts to a direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing audio to pass through while retaining instance and settings. It can also be toggled with the `b` key (does not work while MML input screen, Browse patches/Browse effects are open, or while typing). When bypassed, connected effects in Routing are grayed out and `Bypassed` is appended.
  - `Sequence` is for instruments. Stopping the Sequence continues audio processing to allow for effect tails.
  - On next launch, the instrument, order of all effects, their respective states, and Bypass status are restored before playback starts. History of one old-format effect can also be loaded. If effect restoration fails, an error is displayed, it reverts to a direct instrument connection, and the saved configuration is retained.
  - Addition, removal, and reordering are saved only if the new connection setup succeeds. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplicated; stereo → mono is left/right average. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause short audio dropouts, and sounding notes/tails may be cut off. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Patches
  - `Browse patches` near `Random patch` or the `T` key opens an independent, maximized patch browser window on the same screen as the main window. It does not change the main window's position or size. It cannot be opened while editing MML. Filter by framed Role → Preset → Search. The left condition area and right Patches list are individually scrollable. The list uses the remaining width and height, usable even when unmaximized or in a narrow window. The top row contains Search, `Solo/Mute`, `Random`, `☰`. The search field uses the remaining width, and `/` key within the browser moves input to the search field. Detailed instrument conditions can be checked by hovering over `Solo/Mute`. Clicking this entire button or the `M` key within the browser opens a vertical Solo/Mute overlay. `OK` or `Enter` confirms all changes. Solo/Mute appears immediately to the right of the instrument name. Re-clicking the selected mode reverts to normal; `Esc`/clicking outside the overlay discards changes. Multiple Solos are treated as OR; Mutes as exclusions. Search uses case-insensitive regular expressions, space for AND, `-condition` for exclusion, and `plugin:` condition (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases for integrated patches can also be searched. Invalid regex results in an error, stopping application, random selection, and condition saving. If 0 results, review conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 results. Role, Preset, and search string are saved on exit and restored on next launch. Preset is restored by name; if the condition is deleted, it reverts to `ALL` for that Role.
  - The list displays patch name cells on a single, non-wrapping line, arranged in multiple columns (left → right → next line) based on width. In a narrow window, it reverts to 1 column. Resizing maintains the selected patch; only column count changes do not trigger a reload. Clicking a patch does not move the list's display position. Arrow keys or `h`/`j`/`k`/`l` move as visually arranged. Left/right moves to adjacent patches (or ends of adjacent rows at row ends), up/down moves to rows above/below, and to the end of a short final row if no patch is directly below. Entering a number first, like `2j` or `10k`, moves that many times (stops at ends; numbers are canceled by non-movement keys). `PageUp`/`PageDown` moves 10 times up/down, and combines with numbers for multiples of that amount. Key movement, Random, filtering, and column count changes only scroll the selected row into view if it's off-screen. If all patches fit, it does not scroll. For single-instrument results, the instrument name is omitted; for mixed, it's retained. Text color indicates only one difference in results. If instruments are mixed, the instrument name part is color-coded by instrument. For single instruments, if Preset is `ALL`, patch names are color-coded by Preset; if that doesn't distinguish, by category. Colors are Monokai-based. If all are the same, normal text color is used. Colors cycle through 5, so distant groups might have the same color. Pink patches prioritize "pink" in their name. Classification folders are omitted; if distinguishing same-named patches, necessary paths are retained. Dexed program specification is maintained. Hovering shows the full display path, source path, valid category, and merged count if 2+. Load/Size are not always shown in the list; `no category`/`1 merged` are also not displayed. For regular patches, changing selection via mouse click/key movement immediately requests application; the same applies when condition changes select a different candidate. Simply opening does not change the patch. When opened, if the patch applied and sounding via Browse patches/Random patch is in the current results, that patch is selected and scrolled into view (not reapplied). The sounding patch is saved on exit and carried over as the restored patch on next launch. Adding/removing effects or Random effect preserves this carry-over. If the sounding patch is not in the results or unknown, the selection from the last close is maintained (unselected on first open after launch). After changing patches via Favorites/History or instrument load/remove, the sounding patch is unknown, and selection is cleared. Key movement starts from the selection on opening; if unselected, it moves to the beginning. The next selection can be made during preparation; unstarted requests are replaced by the latest one. On successful regular application, duplicate patch names or fixed margins at the bottom are not displayed. Only if selected and applied patches differ is a brief message shown, indicating that the previous patch might continue to sound for unconfirmed pink patches, etc. Pink Space prompt and application failure/save errors are displayed.
  - Pink patches with sample capacity of 64,000,000 bytes or more are not automatically applied (unknown capacity is treated as normal). After selection, `Space` opens a confirmation overlay for instrument, patch, and capacity. Pressing `Enter` within this overlay starts preparation/application. `Esc` during confirmation cancels without changing. Once applied, a patch still requires re-confirmation. A progress overlay is shown during loading; cancellation during this is not possible. `Space`/`Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is the same as `OK`).
  - `Random` or the `R` key within the browser selects evenly from the displayed results. Shortcuts `T`/`R`/`M`/`/`/`h`/`j`/`k`/`l`/numbers/`Enter` do not work while typing. `R`/`M` do not work while browser overlays/save menu are open. For pink patches, only selection is made; the same Space → Enter confirmation is needed. The outer Random patch continues to randomly select evenly from all available patches. `Esc`/`Enter` or OS close of the browser window closes only the browser, preserving the main window and the last successfully applied patch, discarding unstarted requests and old preparation results. Already started application processes are completed. `Esc`/clicking outside the overlay when confirmation overlay, Solo/Mute, or save menu are open closes only the child, not passing the same input to the browser or background operations. OS close closes the entire browser. It can be reopened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to a Role via `Save condition to Role` in the hamburger `☰` menu, and deleted via `Delete` in Preset. Adding from ALL goes to Etc. They are handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` in the cat config directory. On corrupted load, the original file is protected, writing is stopped, and save failures are displayed on screen. Role/Preset/Search/Solo/Mute are retained during the same app launch; only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared/migrated.
  - Browse patches uses the same catalog, supported CLAP instruments, state application, and failure recovery as Random patch. It maintains global MML/Velocity/CC1 selection, effect order/state/Bypass, and saves the state for next launch after success. Favorites/History are not automatically added. Successful application and save failures are distinguished. Progress of preparation/application is updated even when child windows are focused.
  - `Random patch` in `Sequence` evenly selects all patches from the existing cmrt catalog (Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, Vaporizer2 CLAP patches) that match installed instruments and format+ID. It is not an even selection per instrument; Dexed, having more candidates, will be selected more frequently. The tooltip displays target instruments and total candidates. If no matching candidates, the button is not displayed.
  - The Random patch catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` read-only. Random patch does not include VST3 fallback, effect patch selection, or unknown instruments. For effect preset selection, use Random effect/Browse effects in the effect routing section. Dexed retains program specification within the cartridge. Floe/sforzando external samples are needed at their original location; there is no function to duplicate/save samples.
  - In environments without cmrt and thus without the `catalog.json`, Surge XT's patch list is obtained from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) on each launch and saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json` for Browse patches/Random patch candidates. If it cannot be obtained, the last saved list is used; if that's also missing, Random patch is not displayed. A loading message appears until retrieval finishes (max 10 seconds). The list was created from Surge XT 1.3.4; Surge XT of different versions may not show added patches, and selecting a missing patch will display preparation failure. If `catalog.json` exists but is unreadable, it does not switch. The reason is in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - During Random patch preparation, current playback continues, and conflicting load/scan/patch operations are disabled. There might be a wait for sample loading for Floe etc. If stopped, it remains stopped after application; if playing, the current phrase restarts from the beginning. Global MML/Velocity/CC1 selection and effect order/state/Bypass are maintained. After success, the applied state is saved for next launch; Random patch itself does not add Favorites/History.
  - If preparation/application fails, the reason is displayed, and no other patch is automatically selected. The original configuration/state/playback state is restored, and instances where state restoration also failed are prevented from saving. Save failures after patch application are displayed as "applied". Instrument-specific preparation/reflection conditions and validation scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - `★ Add favorite` for each plugin allows saving multiple instances of the current patch and settings as favorites. For instruments, playback pattern, stopped selection, and confirmed MML/chord text are also saved. For effects, playback settings are not saved. Names are automatically generated and can be changed later.
  - When a different plugin is `Load`ed, the replacing plugin's patch, settings, and playback pattern are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. Reloading the same plugin or restoring on startup does not add to History. The instrument's confirmed MML/chord text and stopped selection are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is aborted.
  - During manual/auto save, duplicates of the same plugin, role, playback pattern, confirmed MML text, and state are checked, and existing items and names are reused. In History's auto-record, only a difference in stop state is treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - [Plugin-specific processing policy/list (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides product-specific exceptions, reasons, scope, and limitations. For confirmed TyrellN6 CLAP state format, compressed parts that change only with playback are excluded from duplicate judgment, and patches with identical text patch settings are grouped. Confirmed PCore `UI_op=9/10` and empty lines at the end of text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed Vaporizer2 CLAP format, tiny differences in MSEG time/coordinates (absolute difference for specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less. Details in [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md)) caused by loading/saving are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as original.
  - Switch lists with the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order; new additions are placed at the top. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON enables reordering with `↑`/`↓` at the end of the row, saving immediately. Row click loading/Effect removal works as before. `Sort by plugin name` sorts once by plugin name in ascending order and saves (case-insensitive, preserving relative order for same-named items). Manual adjustments are still possible afterward. Reordering is disabled while filtering, and reorder mode is OFF on startup. See [Decision and Specification (ADR 0017)](docs/adr/00017-manual-favorites-order.md).
  - History is in order of most recent registration. If determined to be the same patch, the saved patch is reused, its registration time updated, and moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are preserved. All old-format items are migrated to History, and all items not starting with `auto ` are also kept in Favorites (because old formats do not record the origin of auto-saves, this ensures all renamed auto-saves are included in History). See [Policy and Reason (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus. Renaming/deleting is done in Favorites.
  - The global settings `☰` has `Show only CLAP for duplicate plugins`, which is ON by default. If a CLAP of the same name, vendor, and type exists, VST3s in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/vendor; if name or vendor is empty, both are shown). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on next launch. Favorites and load/restore formats are not changed.
  - In both tabs, click the name/image to load, and click again to disconnect a connected effect. The currently loaded/connected row is highlighted, and the color persists during effect bypass. `Load` / `Add` / `Remove` buttons in Plugins also perform these operations.
  - Favorites are displayed one item per line. Long names and supplementary info are truncated; hover to see the full text. Waveform/spectrum are displayed below Routing on the right; the right panel width can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display between bottom-right / bottom of screen (default is bottom-right on startup).
  - For sforzando CLAP's confirmed state format, identical patches are not duplicated based solely on save counter and differences in note/CC1 playback values; existing favorites are reused. Differences in patch settings are preserved. See [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` label is displayed to the left of each row. Clicking a favorite name restores its patch/settings. For instrument favorites, the saved confirmed text and playback pattern selection are restored. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites in the old format without text retain current text; old-format stopped items maintain current selection. If conversion or state load/connection fails, the original text/selection/playback state is restored, and failed content is not saved to the session. Instrument favorites maintain connected effects; effect favorites maintain instrument and MML. An instrument is required to call an effect.
  - Connected effect favorites display `Connected - click again to remove`. Clicking the name again removes that effect. This applies even if settings are edited after calling or while bypassed. The instrument, other effects, and saved favorites remain. Another patch of the same format + ID switches settings at the current position; unconnected effects are added to the end. The selected favorite for each effect is also restored on next launch.
  - `…` at the end of the row allows renaming/deleting. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPLDATA%\cat-plugin-player\favorites\`. Short audio dropouts may occur during save/restore. The savable scope is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` for the oscilloscope is unchecked, spectrum, oscilloscope, and Lissajous appear side-by-side at the bottom of the screen.
  - The Lissajous displays the output of the most recent 50ms, reflecting effects and Bypass, rotated 45° (max 4096 samples). A common auto-scale preserves volume difference and aspect ratio; in-phase at the same level results in a vertical line, out-of-phase in a horizontal line, and sine waves with phase difference form ellipses or circles. Left-only output shows a top-left to bottom-right diagonal line, right-only output a top-right to bottom-left diagonal line, mono a vertical line, and silence a central dot.
  - It updates independently of notes, cycle counts, or trigger settings. `Show analysis labels` displays headings. Axes and drawing methods can also be checked in the graph tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation for the last 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 is weak correlation, -1 is out-of-phase. Negative values are shown in red, serving as a guideline for cancellation when mixed to mono. It shows "—" for silence or if one side is silent.
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen displays L (green) and R (blue) outputs, reflecting effects and Bypass, overlaid. Mono displays the same waveform for both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and you can select `1`, `2`, `4`, `8 cycles`. After Note Off, the tail is displayed based on the last note. Output with chords or detuning may not perfectly repeat within that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel. If the left channel is silent, the right channel is used. If no crossing, it displays from the beginning.
  - `Similarity` sweeps the buffer from the beginning by the display width, 1 sample at a time, to find the position with maximum correlation to the previous display. Ties prefer the earlier position, and the same start position is used for both left/right. For the first display or if the comparison target is silent, zero cross is used.
  - Comparison history is reset when a note is sounded, playback pattern changes, display cycle count changes, trigger method changes, or the audio stream changes. A waiting display appears until sufficient data is collected.
  - Exploration is done in a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, history is reset.
  - Default display settings on startup are 4 cycles and zero cross. Display settings are not saved.

# Miscellaneous

This text was generated by AI and is hard to read. I plan to fix it later.

## Window Position and Size Settings

You can specify the main window's launch position/size and the plugin GUI's placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved in `status.json` in the same directory; normal saving does not modify the TOML. If states were mixed in an old TOML, they are migrated on first launch: the original file is saved as `config.toml.before-status-migration.bak`, then state items are removed. Comments, formatting, and unknown items in settings are preserved; independent comments attached to removed items remain at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

`x` and `y` for the main window are logical coordinates and are used as the launch position if both are specified. If omitted, it uses the traditional initial position. `width` and `height` are the inner width/height (logical coordinates) of the main window. Either can be specified alone; if omitted or if values are 0 or non-finite, they revert to width 900, height 600. Changes are reflected on next launch. The plugin placement method is currently only `right_then_bottom_right`; omitting it behaves the same. GUI size and placement are calculated based on actual window frames, screen work area, and DPI. Automatic saving of positions moved manually for the main window or GUI, and per-plugin position specification are not implemented.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this app's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` are always locally referenced from Cargo.toml. The play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to the conventional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` needs to re-export them. The patch browser uses public Role/Preset/filter APIs and tui-core's sample capacity determination. Random effect and Browse effects use play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions; Browse effects' search uses tui-core's search conditions. Insufficient checkout or API will result in build errors, and there will be no automatic fallback to Git versions.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. UAPMD's location during build is specified via environment variable. `config.toml` is not read as a build setting.

Do not use `cargo install`; directly run the exe generated in the default `target/release`. Builds use local code (including uncommitted changes) and existing `target/shim`, but source changes may require re-configuration/re-build. If `target/shim` was created with a different version like VS 2022, it will continue to build with that version, so delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe, extracted to a cache directory on GUI launch, and then loaded.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency structure and operational policy. Local TUI switching is not needed for building in cat. If building across repos with TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; 'off' is a manual task. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. When validating, record the HEAD and diffs for each repo.

Launch the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- ※ May change depending on mood.
- Enhance the strengths not found in cmrt, such as VST3 and GUI support.
  - Link GUI edit results with cmrt.
    - Load and use edited patches and effects in cmrt.
  - Play around with egui for various things.
    - Keyboard display?
    - Drum sequencer?
    - SMF visualization, playing multiple SMFs together, swapping patches and effects?
    - Should these be separated into different apps, or is it better to do them in this app given patch/effect swapping? What workflow is envisioned?
- Utilize various cmrt crates for diverse functionalities.
  - ※ This will also refine the crates and provide benefits through their use, making it a win-win.
  - CLI import / export
    - cmrt's keyboard screen can be triggered from CLI, specifying patch, effect, arpeggio, etc., to play instantly. Enable the same for this app (to the level where just changing `cmrt` to `cat-plugin-player` works).
      - Frequent breaking changes are fine. Manual migration later is possible. Better than being stuck and hindering progress for fear of change.
  - SMF import / export
    - Currently, recording audio plugin names and patch names to SMF output is implemented; expand upon this.

# Concept, What We Aim For
- Instrument
  - For example, a synthesizer with a MIDI keyboard makes a sound when you turn it on and press a key.
    - We aim for that level of ease of use.
      - Minimize operations until sound is produced.
- Sounding
  - We aim to maintain "it sounds on the author's environment" as much as possible.
- Immediate Sound
  - From app launch to sound production, an experienced time of 0.2 seconds. Playback is prioritized for speed.
    - Processes like GUI display preparation are performed after sound production.
      - Sound playing before GUI display might be considered the sound version of a splash screen.
    - It is necessary to run the release-built exe directly (cargo invocation involves timestamp checks, and debug builds have debug code, both adding to launch wait time).
- Educational Use
  - More oriented towards educational use than commercial.
- Experimentation, Exploration, Personal Use
  - Frequent breaking changes will occur.

# Random Thoughts Corner
- ※ Related to the concept.
- I love audio plugin preset patches.
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It's the aggregate of each layer from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are the plugin host and playback UI.
        - This is a layer where "vibe coding" can be applied.
        - Building this can improve UX.
- For whom is this instrument?
  - Cats, babies.
    - Meaning, we target beginners, extending the range to include cats and babies (e.g., Android tablets).
  - ※ This is a metaphor. Actual operation on Android tablets is unconfirmed. The likelihood of effort to make it work on Android if it doesn't is also low.

# Out of Scope, Not Aimed For
- Robustness
  - Absolutely no bugs, the app never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Functionality
  - Equipped with all functions. Usable for all purposes.
- Convenience
  - Extreme pursuit of convenience and ease of use.
- Performance
  - High-speed operation without CPU load in all environments. Audio never drops out.
- Requests
  - Immediate response to all user requests.

# License
- MIT