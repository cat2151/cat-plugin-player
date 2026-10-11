# cat-plugin-player

A lightweight application that allows you to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and How This App Solves Them
- Audio plugin sounds are amazing!
- To play them, it's often necessary to launch a DAW and perform setup.
- Lightweight apps for casual playing sometimes required account registration to obtain, or their user experience (UX) wasn't quite right for me.
- Nowadays, if you have a desired UX, you can achieve it through vibecoding.
- So, I decided to do just that.

# Installation, Browsing Patches & Effects

If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) for you, as shown below. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If you haven't installed an audio plugin, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as described above.
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button automatically starts playback after the plugin is loaded.
  - It can play chord progressions like `F-G-Em-Am` and MML (Music Macro Language) like `cde`.
- GUI
  - You can display the plugin's GUI by pressing the `Show UI` button.
- Patches
  - To change patches, you can easily select them using a unique patch browser (supported by some plugins only).
  - For plugins not supported by the patch browser, you can select patches using the plugin's own GUI.
- Effects
  - To change effects, you can easily select them using a unique effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects using the plugin's own GUI.

※↑This section is written by a human. AI is prohibited from writing here.

# Features (Detailed additions by AI. Planning to refine later as it's hard to read.)

Oscilloscope display methods and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Detection
  - If plugin detection succeeds but no instruments are found, it will display a prompt to install the free Surge XT via winget. It will also show this if only effects are present. The prompt will be in Japanese if Windows' display language is Japanese, otherwise in English. Even if you choose `Later`, it will appear every time the app launches as long as no instruments are available.
  - `View Terms of Service` opens Surge XT's terms (GPLv3) in a browser. Pressing `Agree to Terms and Install` executes winget and displays the stages (`1/4 Preparing`, `2/4 Downloading`, `3/4 Installing`, `4/4 Loading`) and elapsed time. If a Windows confirmation screen (User Account Control) appears at 3/4, select "Yes". Upon success, it automatically rescan the plugin list, loads Surge XT (CLAP), and starts playback, similar to `Load`. In this case, previous instrument/effect restoration is not performed. Even if you close the prompt, installation continues, and upon completion, it performs a rescan and load.
  - If winget is not available or installation fails, it displays the winget message and manual download link, and logs it to `error.log`. Only Surge XT is included, and no download progress is shown. The reason for this choice is in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- GUI
  - The first time a GUI is opened, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG size that fits the list display frame and screen zoom level, while preserving the aspect ratio. It saves to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching name + manufacturer (ignoring leading/trailing spaces and case) share the same image for VST3 / CLAP. If name or manufacturer is missing, they are distinguished by format + ID.
  - Previously saved format-specific images are also automatically migrated and used as shared images based on list and favorite identification information. Patch state and favorite load destinations are still distinguished by format + ID.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while preserving the aspect ratio. Hovering over the image allows confirmation at the saved size. Plugins without a GUI display a temporary icon of the same height.
  - The GUI prioritizes positioning to the right of the main window, aligned with its top edge, within the screen's working area. If it doesn't fit, it's placed at the bottom right of the main window's screen working area. It's positioned upon display and when the GUI is resized, but does not constantly follow the main window's movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the working area; if its height is greater, its top edge aligns.
  - For blank images or failed captures, it retries at 1-second intervals for up to 20 seconds from the GUI display. Saved blank images are also treated as uncaptured, and opening the GUI replaces them with a valid image. After success, it does not recapture.
  - If capture fails after retrying, an error is displayed. Opening the GUI again retries. For GUIs that don't support capture, the temporary icon remains. If you want to retake or if the saved PNG is corrupted, delete the shared PNG for that plugin and any remaining old format-specific PNGs, restart the app, and open the GUI (old PNGs are left during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on the next launch. Stop state, selected playback pattern while stopped, and confirmed MML/chord text are also restored. Unconfirmed edits are not saved.
  - CLAP Note On/Off messages are sent according to the input format supported by the plugin. This includes sforzando CLAP, which only accepts MIDI. The scope of the fix and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. The arrows loop at the ends of the types, and `Stop` is not included in the types. Switching during playback releases all notes in the current phrase and starts the new phrase from the beginning. If stopped, selecting a type does not start playback; `Play` starts it and `Stop` stops it. In the main window, the `Space` key also toggles start/stop (except when typing text). The selected playback pattern (including MML/chords) and stop state are restored on the next launch.
  - To audition MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). An example of a single note is `t120 c8r8d4e4r4`, and a chord progression example is `C F G C`. Both are accepted in the same input field; chord notation conversion is attempted first. Uppercase `C` denotes a chord, lowercase `ceg` is MML.
  - Input is confirmed with `Enter`/`Confirm` and canceled with `Esc`/`Cancel`. Cancellation or conversion errors maintain the previous valid phrase. `Space` and `i` within the input screen are used for text input; `Space` for playback/stop is disabled.
  - Confirmed phrases are looped with `Play`/`Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses overall normal playback. If playback was active just before opening, after confirmation, the new phrase plays from the beginning; after cancellation, the original playback pattern resumes from the start. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. It can also be switched to normal playback patterns. Immediately after confirmation, the text, selection, and playback state are saved; upon cancellation, the restored playback state is also saved. It's restored on the next launch or from instrument favorites/History. Unconfirmed text is not saved, and cancellation or conversion failure preserves the previous confirmed text.
  - Changing input or moving the cursor auditions the single note/chord at that position once. This also applies to mid-edit and paste. Moving or redrawing within the same pronunciation unit does not re-trigger playback. Previous tempo, octave, and note length are reflected, and no sound is produced at rests or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the audition sound ends. The audition sound is released with the next pronunciation unit, cancellation, confirmation, or instrument change. Audition does not alter the confirmed text or normal selection.
  - For MML loop playback, the Velocity for pronunciation prioritizes the `Velocity` field setting over any in-input specifications. CC1 can also be controlled from the `CC1 modulation` field. If the event capacity is reached with a very dense phrase, active notes are released and pronunciation stops. You can restart with `Stop` -> `Play`.
  - When changing patches via Favorites/History, Browse patches, Random patch, or Random effect, the stop/play state is maintained. During playback, it plays from the beginning of the phrase and does not carry over mid-phrase waits from before the change. Leading rests specified in MML are preserved. While the patch browser or effect browser is displayed, `Space` for playback/stop and MML input initiation are suppressed. Patch changes made within the plugin's own GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is rebuilt (e.g., on load, state restoration, connection changes), to preserve the initial sound, it waits for the first phrase to start while continuing normal audio processing. At 48kHz, this wait is about 21ms for Six Sines and 85ms for TyrellN6. MML's leading rests are added on top of this. For application conditions and real-world verification, see [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases the value in the order of Note On within the phrase, then decreases it in the next loop, repeating this action. Changes are reflected from the next note without restarting the phrase and are restored on the next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or dropdown. The initial value is 0. `sweep` repeats a change from 0 to 127 over 2 seconds, then 127 to 0 over the next 2 seconds. The selection is restored on the next launch.
  - Guitar open strings (MIDI note numbers 40, 45, 50, 55, 59, 64 - E, A, D, G, B, E) are Note On sequentially at 125ms intervals. 2 seconds after the last Note On, all 6 notes are simultaneously Note Off, then it repeats after 0.5 seconds.
  - On `Load` success, it memorizes the `Sequence` type selection, start, and stop times. Memory persists even after stop or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - On exit and `Remove`, binary states such as patches and parameters published by the plugin via the UAPMD state API are saved for each plugin in `%LOCALAPPDATA%\cat-plugin-player\states\`. On the next launch or when the same plugin is reloaded, they are restored before playback begins.
  - An instrument `Load` replaces only the instrument, maintaining connected effects and settings. It saves the state of the plugin being replaced, then switches after successfully generating, restoring, and connecting the new plugin. If it fails, it maintains the original configuration and displays an error.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only obtains component state. It does not include external sample duplication or saving on abnormal termination.
  - On startup, it directly loads the previous instrument and effects, restores their states, and then starts playback. Afterwards, it initializes the GUI and scans the list in the background.
  - If direct loading fails (e.g., due to old history or file moves), it restores from the scan results after the GUI is displayed. Old history is updated once played.
  - If the main instrument is CLAP Floe, when audio processing is rebuilt (e.g., on load or patch restoration), it delays the start of the first phrase by 100ms. MML's leading rests are added on top of this. This is a temporary measure for observing symptoms where initial sounds disappear, and resolution in actual use is unconfirmed. For application conditions and review policy, see [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md).
  - For startup timing measurement methods and results, see [docs/startup-timing.md](docs/startup-timing.md).
- Effect Routing
  - Serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent ones. Connected effects can be removed one by one using `Remove`. Duplicate connections of effects with the same format + ID are not allowed.
  - `Random effect` next to `Sequence` randomly selects a preset from installed CLAP effects, using the same catalog and selection targets as cmrt's effect selector. If there are 0 effects, it adds the first one; if 1, it replaces that effect; if multiple, it uniformly selects one of the connected effects to replace. Repeated presses do not increase the number of effects and maintain the order, settings, and Bypass state of other effects. To avoid duplicate connections, format + IDs already connected (excluding the target for replacement) are excluded from candidates. If no suitable candidate is available for the target slot, it displays a reason and makes no change.
  - Random effect requires an instrument. Candidates are obtained from default effect/preset locations scanned by the play-server shared core, and no fallback to VST3 is made. During preparation, current playback continues, and loading, scanning, and patch operations are disabled. While stopped, it remains stopped after application; while playing, it restarts the current phrase from the beginning, maintaining overall MML, Velocity, and CC1 selections. In case of generation, application, or connection failure, it reverts to the original configuration, without automatic re-selection of other candidates. After success, the applied state and connection are saved for the next launch, and Favorites/History are not added. Failure to save is displayed as "applied".
  - `Browse effects` below `Random effect` or the `E`/`X` keys open a separate maximized effect browser window, allowing you to select presets from a list. It cannot be opened while editing MML. Filter by Category → Kind on the left and by search. The classification and selection targets are the same as cmrt's effect selector. Search uses case-insensitive regular expressions, space for AND, `-condition` for exclusion, and applies to preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regular expressions during input display an error and maintain the previous list. The top line displays Search, Target frame, `Random`. `/` within the browser moves input to the search field, `R` selects randomly from filtered results. Multiple column display in the list, navigation with arrow keys / `h` / `j` / `k` / `l` / number for repetition / `PageUp` / `PageDown`, and closing with `Enter` / `Esc` are the same as Browse patches. In results with mixed effects, effect names are added and colored per effect; if Kind differs, preset names are colored per Kind.
  - The target frame is chosen from `Add new effect` or `Replace n` of a connected effect. When opened, it defaults to the frame last written in this browser; otherwise, the last effect; if no effects, `Add new effect`. Applying via `Add new effect` makes the added effect the target, and subsequent selections replace that frame, so the number of effects does not increase by continuing to select. If the target frame is removed, the last effect becomes the target. Presets of effects already connected to frames other than the target are displayed in gray, and selecting them displays a reason and does not change. Presets applied to the target frame via this browser are marked with `▶`, and opening it selects that preset (not reapplying).
  - Clicking/key-navigating or filtering to select a different candidate immediately requests application. Simply opening does not change the effect. You can select the next even during preparation, unstarted selections are replaced by the latest one, and closing cancels them. An instrument is required, and playback continuation during preparation, reverting on failure, and saving for next launch are the same as Random effect. Category, Kind, search string, list selection position, and target frame are saved on exit and restored on next launch. Key navigation starts from the restored selection. Category/Kind are restored by name; if missing, they revert to `ALL` in that field. Selection is restored by preset; if missing from the catalog, it becomes unselected, and key navigation starts from the beginning. The target frame is restored by its position in the chain; if no effect exists at that position, it defaults to the frame at opening.
  - `Routing` displays the connection order. You can change the order by dragging the effect's `↕` button up or down. While dragging, the moving effect name and insertion line are displayed. Instruments are not subject to reordering. You can edit patches or effects using `Show UI` for each plugin. `Instrument`/`Effect` are displayed with the same type color as the left-side list.
  - Removing all effects reverts to a direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing it to pass through while retaining instances and settings. It can also be toggled with the `b` key (disabled in MML input screen, Browse patches/Browse effects display, and when typing text). While bypassed, connected effects in Routing are displayed in gray, with `Bypassed` appended.
  - `Sequence` is for instruments. Stopping the sequence continues audio processing to allow effect tails to decay.
  - On the next launch, it restores the instrument, the order of all effects, their respective states, and the Bypass state before starting playback. A history of one old-format effect can also be loaded. If effect restoration fails, it displays an error, reverts to a direct instrument connection, and retains the saved configuration.
  - Additions, removals, and reordering are only saved if the new connection successfully prepares. If it fails, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono to stereo is duplication; stereo to mono is left/right average. Auxiliary inputs are silent.
  - There may be brief audio dropouts when changing connections or Bypass, and active notes and tails may be cut off. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Patches
  - `Browse patches` near `Random patch` or the `T` key opens a separate, maximized patch browser window on the same screen as the main app. The main app's position and size are unchanged. It cannot be opened while editing MML. Filter by Role → Preset → Search with frames, and the left condition area and right Patches list are individually scrollable. It uses the remaining width and height for the list, allowing operation even when unmaximized or in a narrow window. The top line displays Search, `Solo/Mute`, `Random`, `☰`. The search field uses the remaining width, and the `/` key within the browser moves input to the search field. Details of instrument conditions can be checked by hovering over `Solo/Mute`. Clicking this entire button or the `M` key within the browser opens a vertical Solo/Mute overlay, and `OK` or `Enter` confirms them all. Solo/Mute appear immediately to the right of the instrument name. Re-clicking the selected mode reverts to normal, `Esc`/clicking outside the overlay discards changes. Multiple Solos are treated as OR, Mutes as exclusions. Search uses case-insensitive regular expressions, space for AND, `-condition` for exclusion, and `plugin:` condition (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases for integrated patches can also be searched. Invalid regular expressions result in an error, stopping application, random selection, and condition saving. If 0 results, review conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 results. Role, Preset, and search string are saved on exit and restored on next launch. Preset is restored by name; if the condition is deleted, it reverts to `ALL` for that Role.
  - The list displays patch name cells as a single, non-wrapping line, arranged in multiple columns (left → right → next row) depending on width. In a narrow window, it reverts to 1 column. Resizing maintains the selected patch, and only changing the number of columns does not reload. Clicking a patch does not move the display position of the list. Arrow keys or `h`/`j`/`k`/`l` move as they appear visually. Left/right move to previous/next patch (to the end of the adjacent row at row ends), up/down move to rows above/below, and to the end of a short final row if no patch is directly below. Typing a number first, like `2j` or `10k` (Vim-style), moves that many times (stops at ends; numbers are canceled by non-movement keys). `PageUp`/`PageDown` move 10 times up/down, and combined with a number, they move that multiple. Key movement, Random, filtering, and column count changes scroll to a visible position only if the selected row is off-screen. If all patches fit, it does not scroll. For single-instrument results, the instrument name is omitted; for mixed results, instrument names are retained. Text color indicates only one difference within the results. If instruments are mixed, the instrument name part is colored per instrument. For a single instrument, if Preset is `ALL`, patch names are colored per relevant Preset; if that doesn't distinguish, then per category. Colors are Monokai-based. If all are the same, normal text color is used. Colors repeat in a sequence of 5, so distant groups may share the same color. Pink patches prioritize pink for their patch names. Classification folders etc. are omitted, and necessary paths are retained to distinguish same-named patches. Dexed program specification is maintained. Hovering shows the full display path, source path, valid category, and count for 2 or more merged items. Load/Size are not permanently on the list, and `no category`/`1 merged` are not displayed. Normal patches immediately request application when selection changes via mouse or key movement, and the same action occurs when a different candidate is selected by a condition change. Simply opening does not change the patch. When opened, if the patch currently playing (applied via Browse patches/Random patch) is in the current results, that patch is selected and scrolled into view (not reapplying). The playing patch is saved on exit and carried over as the restored patch on next launch. Adding/removing effects or Random effect maintains this carry-over. If the playing patch is not in the results or unknown, the selection from when it was last closed is maintained (unselected when opened for the first time after launch). After changing patches by loading/deleting Favorites/History/instruments, the playing patch is unknown, and selection is also cleared. Key navigation starts from the selection at opening; if unselected, it moves to the beginning. The next selection can be made even during preparation, unstarted requests are replaced by the latest one. Upon successful normal application, redundant patch names or fixed whitespace at the bottom are not displayed. A short guide is displayed only if the selected and applied patches differ, indicating that the previous patch might continue playing for unconfirmed pink patches, etc. Pink's Space guide and application failure/save errors are displayed.
  - Pink patches with a sample size of 64,000,000 bytes or more are not automatically applied (unknown size is treated as normal). After selection, `Space` opens a confirmation overlay for instrument, patch, and size, and pressing `Enter` within it starts preparation/application. `Esc` during confirmation cancels without changing. Even once applied, reconfirmation is required. During loading, a progress overlay is displayed, and cancellation is not possible midway. `Space`/`Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is the same as `OK`).
  - `Random` within the browser or the `R` key selects uniformly from the displayed results. While typing text, `T`/`R`/`M`/`/`/`h`/`j`/`k`/`l`/numbers/`Enter` shortcuts are disabled, and `R`/`M` are disabled while browser overlays/save menu are displayed. For pink patches, only selection occurs, requiring the same Space → Enter confirmation. The external Random patch uniformly draws from all available patches, as before. `Esc`/`Enter` or OS close of the browser window only closes the browser, preserving the main app and the last successfully applied patch, discarding unstarted requests and old preparation results. Already started application processes are completed. When confirmation overlay, Solo/Mute, or save menu are open, `Esc`/clicking outside the overlay closes only the child, not passing the same input to the browser or background operations. OS close closes the entire browser. It can be re-opened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to Role via `Save condition to Role` in the hamburger menu `☰`, and deleted via `Delete` in Preset. Adding from ALL goes to Etc. They are handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` in the `cat` config directory. If loading is corrupted, the original file is protected, writing is stopped, and save failure is displayed on screen. Role/Preset/Search/Solo/Mute are retained during the same app session; only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared or migrated.
  - Browse patches uses the same catalog, supported CLAP instruments, state application, and failure recovery as Random patch. MML, Velocity, CC1 global selection, and effect order/state/Bypass are maintained, and after success, the state for the next launch is saved. Favorites/History are not automatically added. Application success and save failure are displayed separately. Progress of preparation/application is updated even when the child window is focused.
  - `Random patch` in `Sequence` uniformly selects from all patches of Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, and Vaporizer2 CLAP patches that match installed instruments and format+ID, from the existing cmrt catalog. It's not a uniform selection per instrument, so Dexed, with more candidates, is selected more frequently. The tooltip displays target instruments and total candidates. If no matching candidates, the button is not displayed.
  - The Random patch catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` as read-only. Random patch does not include VST3 fallback, effect patch selection, or unknown instruments. For effect preset selection, use Random effect/Browse effects in the effect routing section. Dexed retains program specification within cartridges. Floe/sforzando's external samples are required in their original locations; there's no functionality to duplicate/save samples.
  - In environments without cmrt and where `catalog.json` does not exist, Surge XT's patch list is obtained from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) on each launch, saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json`, and used as candidates for Browse patches/Random patch. If it cannot be obtained, the previously saved list is used; if that's also unavailable, Random patch is not displayed. A loading wait display appears until retrieval finishes (up to 10 seconds). The list was created from Surge XT 1.3.4, so increased patches in different Surge XT versions won't appear, and selecting a missing patch will show a preparation failure. If `catalog.json` exists but cannot be read, it does not switch. Reasons are in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - During Random patch preparation, current playback continues, and conflicting load, scan, and patch operations are disabled. There may be a wait for Floe's sample loading, etc. While stopped, it remains stopped after application; while playing, it restarts the current phrase from the beginning. MML, Velocity, CC1 global selection, and effect order/state/Bypass are maintained. After success, the applied state is saved for the next launch, and Random patch itself does not add to Favorites/History.
  - If preparation/application fails, it displays the reason and does not automatically select another patch. It restores the original configuration, state, and playback state, and suppresses saving for instances where state restoration failed. Save failures after patch application are displayed as "applied". Instrument-specific preparation/reflection conditions and verification scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - You can save multiple patch/settings configurations as favorites using `★ Add favorite` for each plugin. For instruments, it also saves the playback pattern, selected item while stopped, and confirmed MML/chord text. For effects, it does not save playback settings. Names are generated automatically and can be changed later.
  - When you `Load` a different plugin, the patch/settings/playback pattern of the replaced plugin are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. It is not added for reloading the same plugin or restoring on startup. Confirmed MML/chord text and selection while stopped for instruments are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is canceled.
  - During manual and automatic saving, it checks for duplicates of the same plugin, role, playback pattern, confirmed MML text, and state, and reuses existing items and names. For automatic History recording, differences only in stop state are treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - [Plugin-Specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides product-specific exceptions, reasons, scope, and limitations. For confirmed TyrellN6 CLAP state format, compressed parts that change only with playback are excluded from duplicate determination, and patches with identical text-based settings are grouped. Confirmed PCore `UI_op=9/10` and trailing blank lines in text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed Vaporizer2 CLAP format, minute differences in MSEG time/coordinates (absolute difference for specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less; see [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md)) caused by loading/saving are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as original.
  - Switch lists with the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order; new additions are placed at the beginning. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON allows sorting with `↑` / `↓` at the end of the row, saving immediately. Loading by clicking a row or removing an effect works as before. `Sort by plugin name` sorts once by plugin name in ascending order and saves (case-insensitive, preserving relative order for same-named items). Manual adjustments are still possible afterwards. Sorting is disabled while filtering, and reorder mode is OFF on startup. See [Adoption History and Specifications (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is in order of most recent registration. If the same patch is detected, the saved patch is reused, its registration time is updated, and it moves to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are retained. All old-format items are migrated to History, and items not starting with `auto ` are also kept in Favorites (because old format did not record the origin of auto-saves, thus all renamed auto-saves are included in History). See [Policy and Reasons (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus. Renaming/deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If there are CLAP plugins with the same name, manufacturer, and type, VST3 plugins are hidden from the Plugins list (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both formats are displayed). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and retained on next launch. Favorites and the format loaded/restored are not changed.
  - On both tabs, clicking names/images loads them, and clicking again removes connected effects. The currently loaded/connected row is highlighted, and the color remains even when effects are bypassed. The same operations can be performed using the `Load`/`Add`/`Remove` buttons in Plugins.
  - Favorites are displayed one item per line. Long names and supplementary information are truncated; hover to see the full text. Waveform/spectrum are displayed below Routing on the right; the right panel width can be adjusted by dragging the border. `Audio analysis`'s `On right` toggles analysis display to bottom right/bottom of screen (bottom right on startup).
  - For sforzando CLAP's confirmed state format, identical patches are not duplicated based solely on save counter and note/CC1 playback value differences, reusing existing favorites. Differences in patch settings are preserved. For scope and limitations, see [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md).
  - `Instrument` / `Effect` are displayed colored on the left of each row. Clicking a favorite name restores the patch/settings. For instrument favorites, the saved confirmed text and playback pattern selection are restored. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites in the old format without text retain the current text, and old-format stopped items maintain the current selection. If conversion, state loading, or connection fails, the original text, selection, and playback state are restored, and the failed content is not saved to the session. Instrument favorites maintain connected effects, and effect favorites maintain the instrument and MML. An instrument is required to load an effect.
  - Favorites for connected effects display `Connected - click again to remove`, and re-clicking the name removes that effect. This also applies after editing settings or during Bypass. The instrument, other effects, and saved favorites remain. Another patch of the same format + ID switches settings at the current position, and unconnected effects are appended to the end. The selected favorite for each effect is also restored on the next launch.
  - Rename/Delete operations are available from `…` at the end of the row. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Short audio dropouts may occur during saving/restoring, and the scope of what can be saved is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` of the oscilloscope is unchecked, spectrum, oscilloscope, and Lissajous are arranged horizontally at the bottom of the screen.
  - A Lissajous figure rotated 45° displays the most recent 50ms of output (up to 4096 samples), reflecting effects and Bypass. A common automatic magnification preserves volume differences and aspect ratio; in-phase at the same level results in a vertical line, out-of-phase in a horizontal line, and sine waves with phase differences in ellipses or circles. Output only on the left is a diagonal line from top-left to bottom-right, output only on the right is top-right to bottom-left, mono is a vertical line, and silence is a central dot.
  - It updates independently of notes, period count, or trigger settings, and `Show analysis labels` can display headers. Axes and rendering methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation for the most recent 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is out-of-phase. Negative values are displayed in red, serving as a guide to judge cancellation when mixed to mono. It shows "—" for silence or if one side is silent.
- Waveform Display
  - `Oscilloscope` at the bottom of the screen displays the output's L (green) and R (blue) channels superimposed, reflecting effects and Bypass. Mono displays the same waveform on both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and you can choose `1`, `2`, `4`, `8 cycles`. After Note Off, the tail is displayed based on the last note. Output containing chords or detuning may not repeat perfectly at that width.
  - `Zero cross` aligns with the negative-to-positive crossing of the left channel. If the left channel is silent, it uses the right channel; if no crossing, it displays from the beginning.
  - `Similarity` comprehensively scans for the position that maximizes the correlation coefficient with the previous display, from the start of the buffer for the display width, at 1-sample increments. Ties adopt the earlier position, and the same start position is used for left/right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when a note is played, the playback pattern changes, the display cycle count changes, the trigger method changes, or the audio stream changes. A waiting display appears until sufficient data is gathered.
  - The search runs on a separate thread from audio processing. Display updates may be slow for low notes or many cycles. If display data is interrupted, the history is reset.
  - The default display setting on startup is 4 cycles and zero cross. Display settings are not saved.

# Build & Test Procedures
- ※Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for instructions. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI how to install it, and it's easy! The general idea is "easy with AI's help."
  - Building the C++ part (UAPMD and shim) requires Visual Studio 2026 Build Tools (C++ workload, MSVC v145). Other versions like VS 2022 are not targeted for verification. Example installation: `winget install --id Microsoft.VisualStudio.BuildTools --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
- ※Next prerequisite: If Rust is not installed, ask AI how to install it, and it's easy! That's the general idea.
- As mentioned, confirm that `../uapmd` has been built.
- Confirm that an audio plugin (instrument) like Surge XT is installed. For example, verify that Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the window opens.
- Confirm that Surge XT, etc., are listed on the screen.
- Click the Load button.
- Confirm that it loads and sound is produced.

# Miscellaneous

The AI-generated text is hard to read. I'm planning to fix it later.

## Window Position and Size Settings

You can specify the main window's launch position/size and plugin GUI placement in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved to `status.json` in the same directory, and normal saves do not overwrite the TOML. If states are mixed in the old TOML, they are migrated on first launch, saving the original file as `config.toml.before-status-migration.bak` and then removing only state-related entries. Comments, formatting, and unknown entries in the settings are preserved, and independent comments attached to deleted entries remain at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates, used as the launch position if both are specified. If omitted, it uses the traditional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. You can specify only one; if omitted or if values are 0 or less/non-finite, they revert to width 900 and height 600. Changes are reflected on the next launch. The plugin placement method is currently only `right_then_bottom_right`, and the behavior is the same if omitted. GUI size and placement are calculated based on the actual window frame, screen working area, and DPI. Automatic saving of positions moved manually, or per-plugin position specification, is not performed.

## Local Build and Execution

Clone this application, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then run from this application's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` are always referenced locally from Cargo.toml. Play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` requires their re-export. The patch browser uses public Role/Preset/filter APIs and tui-core's sample size determination. Random effect and Browse effects use play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector conditions, while Browse effects search uses tui-core's search conditions. Insufficient checkout or API will result in build errors; automatic switching to Git versions is not performed.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The location of UAPMD during build is specified by an environment variable. `config.toml` is not read as a build configuration.

Do not use `cargo install`; directly run the exe generated in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require reconstruction and rebuilding. If `target/shim` was created with VS 2022 or another version, it will continue to build with that version, so delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe and unpacked to a cache directory for loading when the GUI starts.

For the dependency structure and operational policy of the three repositories, refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md). Local switching for TUI is not necessary for building with `cat`. If building TUI cross-repo itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; `off` is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. When verifying, record HEAD and diffs for each repository.

Launches the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- ※Subject to change based on mood.
- Extend the strengths not present in cmrt, such as VST3 and GUI support.
  - Automated GUI operation?
    - Log GUI operations and reproduce them swiftly in cmrt's TUI? As hints for plugin-specific customization?
  - Play around with egui.
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates.
  - ※This will also refine the crates, and we benefit from using them, so it's a win-win.
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can play patches, effects, arpeggios, etc., with a single command from the CLI. Make the same possible (to the level where it plays by just changing `cmrt` to `cat-plugin-player`).
      - Frequent destructive changes are acceptable. Manual migration is possible later. It's better than becoming stagnant due to fear of change.

# Concept, What We Aim For
- Musical Instrument
  - For example, a synthesizer with a MIDI keyboard makes sound when you power it on and press keys.
    - We aim for that level of ease of use.
      - Minimize operations until sound is produced.
- Sound Production
  - We aim to maintain "it makes sound in the author's environment" as much as possible.
- Immediate Sound
  - From app launch to sound production, an experienced 0.2 seconds. Playback is optimized for maximum speed.
    - Processing for GUI display preparation, etc., happens after sound.
      - Sound playing before GUI display might be considered the sound version of a splash screen.
    - You must run the release-built exe directly (running via cargo incurs timestamp checks, and debug builds have debug code, both adding to launch wait time).
- Educational Use
  - It's oriented more towards educational use than commercial use.
- Experimentation, Exploration, Personal Use
  - We make frequent destructive changes.

# Random Thoughts Corner
- ※Related to the concept.
- I love audio plugin preset sounds.
  - Because they are reproducible.
  - Because they are shared freely, everyone can benefit.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It's a combination of various layers, from audio plugins to the PC keyboard.
    - There are intermediate layers.
      - These are plugin hosts and playback UIs.
        - This is a layer where vibecoding can be applied.
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - That is, expanding the target range to include beginners, like cats and babies (e.g., on Android tablets).
  - ※This is purely a metaphor. Actual operation on Android tablets is unconfirmed. Efforts to make it work on Android, if it doesn't, are also likely low.

# Out of Scope, What We Don't Aim For
- Robustness
  - Absolutely no bugs, never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Usability
  - Extreme pursuit of convenience and ease of use.
- Performance
  - Operates at high speed with no CPU load in all environments. Audio never cuts out.
- User Requests
  - Immediate response to all user requests.

# License
- MIT