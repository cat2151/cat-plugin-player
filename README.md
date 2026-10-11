# cat-plugin-player

A lightweight app for easily playing audio plugins, leveraging [UAPMD](https://github.com/atsushieno/uapmd). Written in Rust.

# Past Challenges and What This App Solves
- Audio plugins offer fantastic sounds!
- Often, playing them requires launching a DAW and significant setup.
- Lightweight apps for easy playback sometimes require account registration or have UI/UX that doesn't quite fit.
- Today, if you have a desired UX, you can achieve it through "vibe coding".
- So, that's what we decided to do.

# Installation, Browsing Patches & Effects

If you don't have audio plugins installed, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as shown below. Easy!

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# Features
- Automatic Installation
  - If no audio plugins are installed, the app will automatically install [Surge XT](https://surge-synthesizer.github.io/) as mentioned above.
- Discovery
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Clicking a plugin loads it and automatically starts playback.
  - It can play chord progressions like `F-G-Em-Am` or MML (Music Macro Language) like `cde`.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI, allowing you to edit patches and effects.
- Patches
  - Each plugin offers a variety of preset patches. You can easily select them using its custom patch browser (supported by some plugins only).
  - For plugins not supported by the patch browser, you can select patches using the plugin's GUI as usual.
- Effects
  - Each plugin offers a variety of preset effects. You can easily select them using its custom effect browser (supported by some plugins only).
  - For plugins not supported by the effect browser, you can select effects using the plugin's GUI as usual.

※↑This section is written by a human. AI is forbidden from writing here.

# Features (Detailed additions by AI. Planning to revise later for readability)

Oscilloscope display methods and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- Discovery
  - If plugin discovery succeeds but no instruments are found, it displays a prompt to install the free Surge XT via winget. It also displays this if only effects are found. The prompt is in Japanese if Windows' display language is Japanese, otherwise it's in English. Even if you choose `Later`, it will show the prompt every time you launch the app until an instrument is installed.
  - Clicking `View Terms of Use` opens the Surge XT license (GPLv3) in your browser. Pressing `Agree to Terms and Install` executes winget and shows the stages and elapsed time: "1/4 Preparing", "2/4 Downloading", "3/4 Installing", "4/4 Loading". If a Windows User Account Control prompt appears at 3/4, select "Yes". Upon success, it automatically re-scans the plugin list, loads Surge XT (CLAP), and starts playback just like `Load`. In this case, it does not restore the previous instrument/effect. Closing the prompt continues the installation, and upon completion, it performs a re-scan and load.
  - If winget is not available or installation fails, it displays the winget message and a manual download link, and logs it to `error.log`. Only Surge XT is included, and no download progress is shown. The reasons for this choice are in [ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md).
- GUI
  - The first time the GUI is opened, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG image that fits the list display frame and screen magnification, preserving the aspect ratio. It is saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`. Plugins with matching names + manufacturers (ignoring leading/trailing spaces and case) will share the same image for VST3 / CLAP. If the name or manufacturer is missing, they are distinguished by format + ID.
  - Previously saved format-specific images are automatically migrated and used as shared images based on list/favorite identification. Patch states and favorite load destinations are still distinguished by format + ID as before.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while preserving the aspect ratio. Hovering over the image shows it at its saved size. Plugins with no GUI displayed show a temporary icon of the same height.
  - The GUI prioritizes placement next to the main window, aligned with the top edge, within the screen's work area. If it doesn't fit, it's placed in the bottom right of the main window's screen work area. This placement occurs when displayed and when the GUI is resized, but it does not constantly follow the main window's movement. The GUI is not scaled down; if its width is greater than the screen, its left edge aligns with the work area; if its height is greater, its top edge aligns.
  - If a blank image or a capture failure occurs, it retries every 1 second, up to a maximum of 20 seconds from GUI display. Saved blank images are also treated as uncaptured, and opening the GUI will replace them with a valid image. It does not re-capture after success.
  - If capture fails after retries, an error is displayed. Opening the GUI again will retry. GUIs that do not support capture will remain with a temporary icon. If you want to retake a screenshot or if the saved PNG is corrupted, delete the corresponding shared PNG and any remaining old format-specific PNGs for that plugin, restart the app, and open the GUI (old PNGs are retained during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next launch. Stopped state, selected playback pattern while stopped, and confirmed MML/chord text are also restored. Unconfirmed edits are not saved.
  - CLAP Note On/Off messages are sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of the fix and validation records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. The arrows cycle at both ends of the types, and "stop" is not included in the types. Switching during playback releases all notes in the current phrase and starts from the beginning of the new phrase. While stopped, selecting a type does not start playback; use `Play` to start and `Stop` to stop. In the main window, you can toggle start/stop with the `Space` key (except when typing text). The selected playback pattern (including MML/chords) and stop state are restored on next launch.
  - For MML/chord progression preview, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). An example of a single note is `t120 c8r8d4e4r4`, and a chord progression example is `C F G C`. Both are accepted in the same input field, with chord notation conversion attempted first. Uppercase `C` will be a chord, lowercase `ceg` will be MML.
  - Confirm input with `Enter` / `Confirm`, and cancel with `Esc` / `Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are used for text input; `Space` for play/stop does not work.
  - Confirmed phrases loop playback with `Play` / `Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses the overall normal playback. If playback was ongoing just before opening, after confirmation, the new phrase plays from the start; after cancellation, the original playback pattern plays from the start. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to normal playback patterns. Immediately after confirmation, the text and selection/playback state are saved; upon cancellation, the restored playback state is saved. These are restored on next launch or from instrument favorites/history. Unconfirmed text is not saved, and cancellation or conversion failure retains the previous confirmed text.
  - Changing input or moving the cursor previews the single note/chord at that position once. This also applies to mid-edit and pasting. Moving within the same pronunciation unit or merely redrawing does not re-trigger the sound. It reflects the previous tempo, octave, and note length, and does not sound at rest or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the preview sound ends. The preview sound is released upon the next pronunciation unit, cancellation, confirmation, or instrument change. Preview does not change the confirmed text or normal selection.
  - For loop playback of confirmed MML, the `Velocity` field setting takes precedence over any velocity specified within the input. CC1 can also be controlled from the `CC1 modulation` field. If the performance event capacity is reached with very dense phrases, currently playing notes are released, and playback stops. You can restart with `Stop` → `Play`.
  - When changing patches via app Favorites/History, Browse patches/Random patch/Random effect/Browse effects, the stop/playback state is maintained. During playback, it plays from the beginning of the phrase and does not inherit any wait times from the middle of the previous phrase. Leading rests specified in MML are preserved. While the patch/effect browser is displayed, `Space` for play/stop and MML input initiation are suppressed in the background. Patch changes via the plugin's native GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is rebuilt due to loading, state restoration, or connection changes, it waits for the first phrase to start while continuing normal audio processing to preserve the initial sound. At 48kHz, the wait is approximately 21ms for Six Sines and 85ms for TyrellN6. MML leading rests are added on top of this. For application conditions and real-world validation, refer to [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - In the `Velocity` field below the playback pattern, you can select `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases the value in Note On order within the phrase, then decreases it in the next loop, repeating the cycle. Changes are reflected from the next note without restarting the phrase and are restored on next launch.
  - In the `CC1 modulation` field, you can select `0`, `127`, `sweep` using left/right arrows or the dropdown. The initial value is 0. `sweep` repeats a cycle of changing from 0 to 127 over 2 seconds, then from 127 to 0 over the next 2 seconds. The selection is also restored on next launch.
  - Guitar open strings send Note On for MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) sequentially at 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Off simultaneously, and this repeats every 0.5 seconds.
  - Upon successful `Load`, and when selecting, starting, or stopping `Sequence` types, the state is remembered. The memory persists even after stopping or `Remove`. Older history/favorites are restored with the traditional 4-note pattern.
  - Upon exit and `Remove`, the binary state of patches/parameters exposed by plugins via the UAPMD state API is saved to `%LOCALAPPDATA%\cat-plugin-player\states\` per plugin. On next launch or re-load of the same plugin, this state is restored before playback begins.
  - Instrument `Load` only replaces the instrument, maintaining connected effects and settings. It saves the state of the plugin being replaced, and switches only after successful generation, restoration, and connection of the new plugin. If it fails, the original configuration is maintained, and an error is displayed.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. It does not include duplication of external samples or saving during forced termination.
  - On startup, it directly loads the previous instrument and effects, restores their states, and then starts playback. Afterwards, it initializes the GUI and scans the list in the background.
  - If direct loading fails due to old history or file movement, it restores from the scan results after GUI display. Old history is updated once played.
  - For the main instrument CLAP Floe, when audio processing is rebuilt due to loading or patch restoration, the start of the first phrase is delayed by 100ms. MML leading rests are added on top of this. This is a provisional measure to observe symptoms of missing initial notes, and resolution on actual hardware is unconfirmed. For application conditions and review policy, refer to [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md).
  - For startup time measurement methods and results, refer to [docs/startup-timing.md](docs/startup-timing.md).
- Effect Routing
  - Serial connection: one instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. Connected effects can be removed one by one with `Remove`. Duplicate connections of effects with the same format + ID are not allowed.
  - `Random effect` next to `Sequence` randomly selects a preset from installed CLAP effects, using the same catalog and selection targets as cmrt's effect selector. If there are 0 effects, it adds the first one; if 1, it replaces that effect; if multiple, it uniformly selects and replaces one of the connected effects. Repeated clicks do not increase the number of effects, and the order/settings of other effects and Bypass are maintained. To avoid duplicate connections, connected format + IDs (other than the target for replacement) are excluded from candidates. If no candidates are available for the target slot, it displays a reason and does not change.
  - Random effect requires an instrument. Candidates are obtained from default effect/preset locations searched by the play-server shared core, and no VST3 substitution is performed. During preparation, the current playback continues, and loading, scanning, and patch operations are disabled. While stopped, it remains stopped; while playing, it restarts the current phrase from the beginning, maintaining overall MML, Velocity, and CC1 selections. In case of generation, application, or connection failure, it reverts to the original configuration, and no automatic re-selection of other candidates occurs. Upon success, the applied state and connection are saved for next launch, and Favorites/History are not added. Save failure is displayed as "applied".
  - `Browse effects` below `Random effect`, or the `E` / `X` keys, opens a separate maximized effect browser window where you can select presets from a list. It cannot be opened while MML is being edited. Narrow down by Category → Kind on the left and by search. The classification and selection targets are the same as cmrt's effect selector. Search uses case-insensitive regular expressions, space-AND, and `-condition` for exclusion, applying to any of preset name, Category, Kind, or effect name (e.g., `hall -plate`, `dragonfly`). Invalid regular expressions during input display an error and retain the previous list. The top row displays Search, Target Frame, and `Random`. `/` within the browser moves input to the search field, and `R` randomly selects from filtered results. Multi-column display, navigation with arrow keys / `h` / `j` / `k` / `l` / numeric repetition, `PageUp` / `PageDown` for movement, and `Enter` / `Esc` to close operate the same as Browse patches. In results where multiple effects are mixed, effect names are appended per effect; if Kinds differ, preset names are color-coded per Kind.
  - The target frame is selected from `Add new effect` or `Replace n` of a connected effect. When opened, it defaults to the frame last written to by this browser; if none, the last effect; if no effects, `Add new effect`. Applying with `Add new effect` makes the added effect the target, and subsequent selections replace that frame, so the number of effects does not increase by continuing to select. If the target frame is removed, the last effect becomes the target. Presets of effects connected to frames other than the target are displayed in gray; selecting them shows a reason and does not change. Presets added to the target frame via this browser are marked with `▶`, and opening it selects that preset (it does not reapply).
  - Clicking/key-navigating or filtering to select a different candidate immediately requests application. Simply opening does not change the effect. You can select the next even while preparing; uninitiated selections are replaced by the latest one, and closing cancels them. An instrument is required, and playback continuation during preparation, restoration upon failure, and saving for next launch are the same as for Random effect. Category, Kind, search string, list selection position, and target frame are saved on exit and restored on next launch. Key navigation starts from the restored selection. Category/Kind are restored by name; if missing, they revert to `ALL` in that field. Selection is restored by preset; if missing from the catalog, it becomes unselected, and key navigation starts from the beginning. The target frame is restored by its position in the chain; if no effect exists at that position, it defaults to the frame when opened.
  - `Routing` displays the connection order. You can drag the effect's `↕` button up or down to change the order. While dragging, the moving effect's name and insertion line are displayed. Instruments are not subject to reordering. Each plugin's `Show UI` allows editing patches and effects. `Instrument` / `Effect` are displayed with the same type color as the left list.
  - Removing all effects reverts to a direct connection from instrument to output. `Bypass effect` targets the entire chain, allowing a pass-through while retaining instances and settings. It can also be toggled with the `b` key (does not work in MML input screen, Browse patches/Browse effects display, or while typing text). While bypassed, connected effects in Routing are shown in gray, with `Bypassed` appended.
  - `Sequence` is for instruments. Stopping the sequence continues audio processing to allow effect tails.
  - On next launch, the instrument, order of all effects, their respective states, and Bypass status are restored before playback. History of one old-format effect can also be loaded. If effect restoration fails, an error is displayed, and it reverts to a direct instrument connection, retaining the saved configuration.
  - Add, remove, and reorder operations are saved only upon successful preparation of the new connection. If unsuccessful, the original configuration and playback are restored, and an error is displayed.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication; stereo → mono is the average of left/right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause brief audio dropouts, and currently playing notes and tails may be cut off. Sidechaining, parallel connections, and host-side Dry/Wet are not supported.
- Patches
  - `Browse patches` near `Random patch`, or the `T` key, opens a separate maximized patch browser window on the same screen as the main app. It does not change the main app's position or size. It cannot be opened while MML is being edited. Filter by Role → Preset → Search with frames, and both the left condition area and right Patches list are scrollable. The list uses the remaining width and height, operable even when unmaximized or in a narrow window. The top row displays Search, `Solo/Mute`, `Random`, and `☰`. The search field uses the remaining width, and the `/` key within the browser moves input to the search field. Details of instrument conditions can be checked by hovering over `Solo/Mute`. Clicking this button or pressing the `M` key within the browser opens a vertical Solo/Mute overlay, confirming changes with `OK` or `Enter`. Solo/Mute appears immediately to the right of the instrument name. Clicking the selected mode again reverts to normal; `Esc` / clicking outside the overlay discards changes. Multiple Solos are treated as OR, Mute as exclusion. Search uses case-insensitive regular expressions, space-AND, `-condition` for exclusion, and `plugin:` condition (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases of integrated patches can also be searched. Invalid regular expressions result in an error, stopping application, random selection, and condition saving. If 0 results, you can re-examine conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 results. Role, Preset, and search string are saved on exit and restored on next launch. Presets are restored by name; if the condition is deleted, it reverts to `ALL` for that Role.
  - The list displays patch name cells, one line not wrapping, in multiple columns depending on width, arranged left→right→next line. In a narrow window, it reverts to one column. Resizing maintains the selected patch, and changing column count alone does not reload. Clicking a patch does not move the list's display position. Arrow keys or `h` / `j` / `k` / `l` move according to the visual layout. Left/right moves to the previous/next patch (end of row moves to the end of the adjacent row), up/down moves to the rows above/below, and to the end of a short final row if no patch is directly below. Typing a number first, like `2j` or `10k` (vim-style), moves that many times (stops at ends; other keys cancel the number). `PageUp` / `PageDown` moves 10 steps up/down, and combined with numbers, it moves by that multiple. Key navigation, Random, filtering, and column count changes only scroll the selected row into view if it's off-screen. If all patches fit, it does not scroll. For single-instrument results, the instrument name is omitted; when mixed, instrument names are retained. Text color indicates only one difference in results. If instruments are mixed, the instrument name part is color-coded per instrument. For single instruments, when Preset is `ALL`, patch names are color-coded per corresponding Preset; if that doesn't distinguish, then per category. Colors are Monokai-based. If all are the same, it's normal text color. Colors cycle through 5, so distant groups might have the same color. Pink patches prioritize pink color for their names. Folder classifications, etc., are omitted; if distinguishing same-named patches, necessary paths are retained. Dexed program specification is maintained. Hovering shows the full display path, source path, valid categories, and merged count if 2 or more. Load/Size are not always shown in the list, and `no category` / `1 merged` are also not displayed. Normal patches immediately request application when selection changes via mouse/key navigation; the same applies when a different candidate is chosen due to condition changes. Simply opening does not change the patch. If, upon opening, the currently playing patch (applied via Browse patches / Random patch) is in the current results, that patch is selected and scrolled into view (it is not reapplied). The playing patch is saved on exit and inherited as the restored patch on next launch. This inheritance is maintained during effect addition/removal/Random effect. If the playing patch is not in the results or unknown, the selection from when it was last closed is maintained (unselected when opened for the first time after launch). After changing patches via Favorites/History, or instrument loading/removal, the playing patch is unknown, and selection is cleared. Key navigation starts from the selection when opened; if unselected, it moves to the beginning. The next selection can be made even while preparing; uninitiated requests are replaced by the latest one. Upon successful normal application, redundant patch names or fixed padding at the bottom are not displayed. Only if the selected and applied patches differ is a brief notice shown, indicating that the previous patch might continue playing for unconfirmed pink patches. Pink Space prompts and application failures/save errors are displayed.
  - Pink patches with a sample size of 64,000,000 bytes or more are not applied automatically (unknown sizes are treated as normal). After selection, `Space` opens a confirmation overlay for instrument, patch, and size; pressing `Enter` within that overlay starts preparation/application. `Esc` during confirmation cancels without changes. Even once applied, a patch requires re-confirmation. During loading, a progress overlay is displayed, and cancellation is not possible. `Space` / `Enter` in the search field, save menu, or Solo/Mute overlay are not used for patch application (`Enter` in Solo/Mute overlay is the same as `OK`).
  - `Random` or the `R` key within the browser selects uniformly from the displayed results. While typing text, `T` / `R` / `M` / `/` / `h` / `j` / `k` / `l` / numbers / `Enter` shortcuts do not work, and `R` / `M` do not work while the browser overlay or save menu is displayed. For pink patches, only selection is done, and the same Space → Enter confirmation is required. The external Random patch uniformly selects from all available patches as before. `Esc`, `Enter`, or OS close of the browser window closes only the browser, preserving the main app and the last successfully applied patch, discarding uninitiated requests and old preparation results. Already initiated processes are completed. When the confirmation overlay, Solo/Mute, or save menu is open, `Esc` / clicking outside the overlay only closes the child, and the same input is not passed through to the browser or background operations. OS close closes the entire browser. It can be reopened, and there is no undo operation to revert to the original patch.
  - Search conditions can be added to Role via `Save condition to Role` in the hamburger `☰` menu and deleted via `Delete` in Preset. Adding from ALL goes to Etc. They are handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` in the `cat` config directory. If loading is corrupted, the original file is protected, writing is stopped, and save failures are also displayed on screen. Role/Preset/Search/Solo/Mute are maintained during the same app session; only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared or migrated.
  - Browse patches uses the same catalog, supported CLAP instruments, state application, and failure recovery as Random patch. It maintains overall MML, Velocity, CC1 selections, and effect order/state/Bypass; upon success, it saves the applied state for next launch. Favorites/History are not automatically added. Application success and save failure are displayed separately. Progress of preparation/application is updated even when child windows are in focus.
  - `Random patch` in `Sequence` uniformly selects from all available CLAP patches for Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, and Vaporizer2 in the existing cmrt catalog that match installed instruments and format+ID. It is not a uniform random selection per instrument; Dexed, having more candidates, will be selected more frequently. The tooltip displays target instruments and total candidates. If no matching candidates, the button is not displayed.
  - The Random patch catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode. Random patch does not substitute VST3, randomly select effect patches, or target unknown instruments. For effect preset selection, use Random effect / Browse effects in the Effect Routing section. Dexed retains program specification within cartridges. Floe/sforzando external samples are required at their original locations; there is no function to duplicate/save samples.
  - In environments without cmrt and where `catalog.json` does not exist, the Surge XT patch list is retrieved on each launch from [cat-music-patterns](https://github.com/cat2151/cat-music-patterns) and saved to `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json` for use as candidates in Browse patches / Random patch. If it cannot be retrieved, the last saved list is used; if that also doesn't exist, Random patch is not displayed. A loading wait display (up to 10 seconds) appears until retrieval finishes. The list was created from Surge XT 1.3.4; Surge XT versions with different builds will not show added patches, and selecting a non-existent patch will display a preparation failure. If `catalog.json` exists but cannot be read, it does not switch. The reasons are in [ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md).
  - During Random patch preparation, the current playback continues, and conflicting load, scan, and patch operations are disabled. There may be a wait for sample loading for Floe, etc. While stopped, it remains stopped after application; while playing, it restarts the current phrase from the beginning. Overall MML, Velocity, CC1 selections, and effect order/state/Bypass are maintained. Upon success, the applied state is saved for next launch, and Random patch itself does not add to Favorites/History.
  - If preparation/application fails, a reason is displayed, and no other patch is automatically selected. The original configuration, state, and playback state are restored, and instances where state restoration also failed are suppressed from saving. Save failure after patch application is displayed as "applied". Plugin-specific preparation/reflection conditions and validation scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - You can save multiple patches and settings as favorites at that moment using `★ Add favorite` for each plugin. For instruments, playback patterns, selections while stopped, and confirmed MML/chord text are also saved. For effects, playback settings are not saved. Names are automatically generated and can be changed later.
  - When you `Load` a different plugin, the patch, settings, and playback pattern of the replaced plugin are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. It is not added for re-loading the same plugin or restoring on startup. Confirmed MML/chord text and selections while stopped for instruments are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is canceled.
  - During manual and automatic saving, it checks for duplicates (same plugin, role, playback pattern, confirmed MML text, state) and reuses saved items and names. For automatic History recording, only differences in stop state are treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - [Plugin-specific processing policy/list (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides product-specific exceptions, reasons, scope, and limitations. For confirmed TyrellN6 CLAP state format, compressed parts that change only with playback are excluded from duplicate checking, and patches with identical text patch settings are grouped. Confirmed PCore `UI_op=9/10` and trailing empty lines in text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For confirmed Vaporizer2 CLAP format, minute differences in MSEG time/coordinates (absolute difference of specified items: time <= 0.00011 ms, normalized coordinates <= 0.000001. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md)) that occur during loading/saving are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - Switch lists with the central `Favorites` / `History` / `Plugins` tabs. Favorites maintain manual order; new additions are placed at the beginning. Calling, re-saving, or recording to History does not change the order. You can filter by favorite name, plugin name, and format.
  - In the Favorites hamburger menu, turning `Reorder favorites` ON allows you to reorder with `↑` / `↓` at the end of the row and save immediately. Row clicks for loading and effect removal work as before. `Sort by plugin name` sorts once in ascending order by plugin name and saves (case-insensitive, same-named items maintain relative order). Manual adjustments are still possible afterward. Reordering is disabled while filtering, and reorder mode is OFF on startup. See [Decision and Specification (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is sorted by most recent registration. If determined to be the same patch, the existing patch is reused, its registration time is updated, and it is moved to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are maintained. All old-format items are migrated to History, and all items not starting with `auto ` are also kept in Favorites (because old formats did not record the origin of auto-saves, ensuring all renamed auto-saves are included in History). See [Policy and Reasons (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename / Delete operation menus. Renaming / deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global `☰` settings is ON by default. If a CLAP plugin with the same name, manufacturer, and type exists, VST3 plugins in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both are displayed). Turning it OFF displays both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and maintained on next launch. It does not change Favorites or the format loaded/restored.
  - In both tabs, clicking a name/image loads it, and clicking again removes a connected effect. The currently loaded/connected row is colored, and the color is maintained even when effects are bypassed. The same operations can be done with the `Load` / `Add` / `Remove` buttons in Plugins.
  - Favorites are displayed one item per line. Long names/supplementary information are abbreviated, and the full text can be viewed by hovering. Waveform/spectrum are displayed below Routing on the right side, and the right side's width can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display to bottom-right / bottom of screen (bottom-right on startup).
  - For sforzando CLAP's confirmed state format, only differences in save counter and note/CC1 playback values do not create new identical patches; existing favorites are reused. Differences in patch settings are preserved. For application scope and limitations, refer to [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md).
  - `Instrument` / `Effect` are displayed colored on the left of each row. Clicking a favorite name restores the patch/settings. For instrument favorites, the saved confirmed text and playback pattern selection are restored. If stopped, it remains stopped; `Play` plays the restored phrase. While playing, it switches to the saved pattern/MML. Old-format favorites without text retain the current text, and old-format stopped items maintain the current selection. If conversion or state loading/connection fails, the original text/selection/playback state is restored, and the failed content is not saved to the session. Instrument favorites maintain connected effects; effect favorites maintain the instrument and MML. An instrument is required to call an effect.
  - For connected effect favorites, `Connected - click again to remove` is displayed; re-clicking the name removes that effect. This applies even after editing settings or bypassing after calling. The instrument, other effects, and saved favorites remain. Another patch of the same format + ID switches settings at the current position, and unconnected effects are added to the end. The selected favorite for each effect is also restored on next launch.
  - `…` at the end of the row allows renaming/deleting. Edits after calling or traditional auto-saves do not overwrite favorites. `Last favorite` indicates the last saved/called favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Saving/restoring may cause brief audio dropouts, and the savable scope is the same as traditional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right analysis panel. If `☰` → `On right` is unchecked for the oscilloscope, the spectrum, oscilloscope, and Lissajous appear horizontally at the bottom of the screen.
  - A 45° rotated Lissajous curve is drawn using the most recent 50ms of output, reflecting effects and Bypass (max 4096 samples). A common auto-scaling maintains volume differences and aspect ratio; in-phase at the same level shows a vertical line, out-of-phase a horizontal line, and sine waves with phase differences show ellipses or circles. Output only on the left shows a diagonal line from top-left to bottom-right; output only on the right shows top-right to bottom-left; mono shows a vertical line; silence shows a central dot.
  - It updates independently of notes, cycle count, or trigger settings, and `Show analysis labels` can display headings. Axes and drawing methods can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation over the most recent 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 is weak correlation, -1 is out-of-phase. Negative values are displayed in red, serving as a guide to judge cancellation when mixed to mono. If silent or one side is silent, it shows "—".
- Waveform Display
  - `Oscilloscope` at the bottom of the screen displays overlapping L (green) and R (blue) outputs, reflecting effects and Bypass. Mono displays the same waveform on both.
  - The display width is determined from the last Note On MIDI note number (A4 = 440Hz), and you can choose `1`, `2`, `4`, `8 cycles`. After Note Off, the last note is still used as a reference to display tails. Output containing chords or detuning may not repeat perfectly at that width.
  - `Zero cross` aligns with the negative→positive crossing of the left channel. If the left channel is silent, it uses the right channel; if no crossing, it displays from the beginning.
  - `Similarity` sweeps through the buffer from the start, one sample at a time for the display width, finding the position with the maximum correlation coefficient to the previous display. Ties prefer the earlier position, and the same start position is used for left/right. For the first display or if the comparison target is silent, zero crossing is used.
  - The comparison history is reset when a note sounds, playback pattern, display cycle count, trigger method, or audio stream changes. It shows a waiting display until sufficient data is collected.
  - Searching is done on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - Default display settings on startup are 4 cycles and zero-cross. Display settings are not saved.

# Build & Operation Check Procedure
- ※Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for instructions. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI how to install it – it's easy! The overall image is "mostly let AI handle it, it's easy!"
  - Building the C++ parts (UAPMD and shim) requires Visual Studio 2026 Build Tools (C++ workload, MSVC v145). Other versions like VS 2022 are not tested. Example installation: `winget install --id Microsoft.VisualStudio.BuildTools --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
- ※Next prerequisite: If Rust is not installed, ask AI how to install it – it's easy!
- As mentioned above, ensure `../uapmd` is built.
- Ensure audio plugins (instruments) like Surge XT are installed. For example, confirm Surge XT sounds in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm the screen opens.
- Confirm Surge XT, etc., are listed on the screen.
- Click the Load button.
- Confirm it loads and sound plays.

# Miscellaneous

This section was generated by AI and is a bit hard to read. I plan to refine it later.

## Window Position & Size Settings

You can specify the main window's launch position/size and the plugin GUI's placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved to `status.json` in the same directory, and normal saving does not overwrite the TOML. If states are mixed in the old TOML, they are migrated on first launch, the original file is saved as `config.toml.before-status-migration.bak`, and only state items are removed. Settings comments, formatting, and unknown items are preserved, and independent comments attached to deleted items are retained at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates and are used for launch position if both are specified. If omitted, it reverts to the traditional initial position. `width` and `height` are the inner width and height of the main window (logical coordinates). Either can be specified; if omitted or set to 0 or less/non-finite, they revert to width 900, height 600. Changes are reflected on next launch. The plugin placement method is currently `right_then_bottom_right` only, and the default behavior is the same if omitted. GUI size and placement are calculated based on the actual window frame, screen work area, and DPI. Automatic saving of manually moved main window or GUI positions, or per-plugin position specification, is not performed.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then run from this app's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` are always referenced locally from Cargo.toml. play-server requires `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to the traditional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` requires their re-export. The patch browser uses the public Role/Preset/filter APIs and tui-core's sample size determination. Random effect and Browse effects use play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions, while Browse effects' search uses tui-core's search conditions. Insufficient checkouts or APIs will result in build errors, and automatic switching to Git versions is not performed.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in a sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The location of UAPMD during build is specified by an environment variable. `config.toml` is not read as a build setting.

Do not use `cargo install`; directly execute the generated exe in the default `target/release`. The build uses local code (including uncommitted changes) and the existing `target/shim`, but source changes may require re-configuration and re-building. If `target/shim` was created with VS 2022 or another version, it will continue to build with that version, so delete `target/shim` before building ([ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)). The shim DLL is bundled with the exe and unpacked to a cache directory for loading when the GUI starts.

For the dependency configuration and operational policy of the 3 repositories, refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md). Local switching of TUI is not required for cat's build. If building cross-repo with TUI itself, use TUI's existing `python scripts/cross_repo_local.py on` procedure; `off` is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the contents of each checkout. When validating, record the HEAD and diff of each repo.

Launch the GUI without arguments. `--help` and `--version` are also available.

# Future Brainstorming
- ※Subject to change based on mood/priority.
- Enhance the strengths not found in cmrt, such as VST3 and GUI support.
  - GUI automation?
    - Log GUI operations and reproduce them swiftly in cmrt's TUI? Provide hints for plugin-specific customization?
  - Experiment with egui.
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates.
  - ※This will also refine the crates and provide benefits through their use, making it a win-win.
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - cmrt's keyboard screen can be triggered from CLI, specifying patch, effect, arpeggio, etc., to play sound immediately. Aim to enable the same functionality (to the level where it works by simply changing `cmrt` to `cat-plugin-player`).
      - Frequent breaking changes are acceptable. Manual migration is possible later. Better than being stuck due to fear of change, hindering progress.

# Concept, What We Aim For
- Instrument
  - For example, a synthesizer with a MIDI keyboard powers on and plays sound when you press a key.
    - We aim for that level of ease of use.
      - Minimize operations to produce sound.
- Sounding
  - We aim to maintain "it sounds on the author's environment" as much as possible.
- Immediate Sound
  - Perceived 0.2 seconds from app launch to sound playback. We prioritize the fastest path to performance.
    - Processes like GUI display preparation also occur after the sound.
      - Sound playing before GUI display might be considered the sound version of a splash screen.
    - You need to run the release-built exe directly (running via cargo adds timestamp checks, and debug builds add debug code, both contributing to startup latency).
- Educational
  - It leans more towards an educational tool than a commercial one.
- Experimentation, Exploration, Personal Use
  - We will frequently introduce breaking changes.

# Random Thoughts Section
- ※Related to the concept.
- I like audio plugin preset patches.
  - Because they are reproducible.
  - Because they are shared freely, everyone benefits.
- I want to provide an instrument UX.
  - I want to provide an instrument UX, not a DAW UX.
- What is an instrument?
  - It's the aggregate of each layer, from audio plugins to the PC keyboard.
    - There are intermediate layers.
      - These are plugin hosts and performance UIs.
        - This is a layer where "vibe coding" can be done.
        - Building this can improve UX.
- For whom is this instrument?
  - Cats, babies.
    - Meaning, we target a broad range towards beginners, including cats and babies (e.g., Android tablets).
  - ※This is purely a metaphor. Actual operation on Android tablets is unconfirmed. The likelihood of effort to make it work on Android if it doesn't is also low.

# Out of Scope, What We Don't Aim For
- Robustness
  - Absolutely no bugs, never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Usability
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed with no CPU load in any environment. Audio will never drop out.
- Requests
  - Immediate response to all user requests.

# License
- MIT