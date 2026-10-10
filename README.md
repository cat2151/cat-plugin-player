# cat-plugin-player

A lightweight application to easily play audio plugins. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

# Past Challenges and What This App Solves
- Audio plugin timbres are amazing!
- To play them, it often requires launching a DAW and preparing for playback.
- Lightweight apps for easy playback sometimes require account registration to obtain, or their operation felt not for me.
- In modern times, if you have a desired UX, you can achieve it through vibe coding.
- That's what we decided to do.

# Features
- Detection
  - Upon launch, it automatically detects installed plugins.
- Playback
  - Pressing the `Load` button automatically starts playback after loading the plugin.
- GUI
  - Pressing the `Show UI` button displays the plugin's GUI.
- Timbre
  - To change the timbre, use the plugin's GUI.

※↑This section is written by a human. AI is prohibited from writing here.

# Features (Detailed additions by AI. Planning to revise later as it's hard to read.)

The oscilloscope display method and constraints are recorded in [ADR 0001](docs/adr/0001-scope-follows-waveform-position.md).

- GUI
  - When the GUI is opened for the first time, it waits 2 seconds for rendering, automatically takes a screenshot, and scales it down to a PNG file with the aspect ratio preserved, fitting the display frame in the list and the screen's zoom level. It is saved to `%LOCALAPPDATA%\cat-plugin-player\plugin-icons\`, and plugins with matching names + manufacturers (ignoring leading/trailing spaces and case) will share the same image for VST3 / CLAP. If name/manufacturer is missing, it's distinguished by format + ID.
  - Previously saved format-specific images are automatically migrated from list/favorite identification information to shared images and used. Timbre state and favorite load destinations are still distinguished by format + ID.
  - Plugin images are displayed in Plugins / Favorites / Routing. Within a row, they are scaled down to fit the button height while maintaining aspect ratio. Hovering over an image displays it at its saved size. Plugins without a GUI displayed will show a temporary icon of the same height.
  - The GUI prioritizes positioning itself to the right of the main window, aligned with its top edge, and within the screen's work area. If it doesn't fit, it's placed at the bottom right of the main window's screen work area. It's positioned upon display and when the GUI size changes, but it doesn't constantly follow the main window's movement. The GUI is not scaled down; if its width is larger than the screen, its left edge aligns with the work area; if its height is larger, its top edge aligns.
  - For blank images or failed captures, it retries at 1-second intervals for up to 20 seconds from GUI display. Saved blank images are also treated as uncaptured, and opening the GUI will replace them with valid images. No re-captures occur after success.
  - If capture fails after retries, an error is displayed. Reopening the GUI will retry. GUIs that do not support capture will remain with a temporary icon. If you want to retake or if the saved PNG is corrupted, delete the shared PNG and any remaining old format-specific PNGs for that plugin, restart the app, and open the GUI (old PNGs are retained during migration).
- Playback
  - The last played instrument, connected effects, Bypass state, and playback pattern are saved to `%LOCALAPPDATA%\cat-plugin-player\status.json` and restored on next launch. Stopped state, selected stopped playback pattern, and confirmed MML/chord text are also restored. Unconfirmed edits are not saved.
  - CLAP Note On/Off is sent according to the plugin's supported input format. This includes sforzando CLAP, which only accepts MIDI. The scope of the patch and verification records are documented in [CLAP Note Format Description](patches/uapmd/clap-note-dialect/README.md).
  - In the top-left `Sequence` pane, you can select the playback type using left/right arrows or the dropdown. The arrows cycle at the ends of the types, and stop is not included as a type. Switching during playback releases all notes in the current phrase and starts the new phrase from the beginning. When stopped, selecting a type does not start playback; `Play` starts, and `Stop` stops. The `Space` key in the main window also toggles start/stop (except during text input). Playback pattern selection including MML/chord and stopped state are restored on next launch.
  - To audition MML/chord progressions, open the input screen with the `i` key or the `Chord` button in `Sequence` (hover to see shortcuts and input methods). An example of a single note is `t120 c8r8d4e4r4`; an example of a chord progression is `C F G C`. Both are accepted in the same input field, with chord notation conversion attempted first. Uppercase `C` indicates a chord, lowercase `ceg` indicates MML.
  - Input is confirmed with `Enter` / `Confirm`, and canceled with `Esc` / `Cancel`. Cancellation or conversion errors retain the previous valid phrase. `Space` and `i` within the input screen are used for text input, and `Space` for play/stop does not work.
  - Confirmed phrases loop playback with `Play` / `Stop`. Rests, note lengths, tempo changes, chords, and trailing rests are preserved. Opening the input screen pauses normal overall playback. If playback was active just before opening, it resumes with the new phrase after confirmation, or the original pattern from the beginning after cancellation. If opened while stopped, it remains stopped after confirmation/cancellation, and `Play` starts overall playback. You can also switch to regular playback patterns. The text, selection, and playback state are saved immediately after confirmation, and the restored playback state is saved upon cancellation. These are restored on next launch or from timbre favorites/History. Unconfirmed text is not saved, and cancellation or conversion failure preserves the last confirmed text.
  - Changing input or moving the cursor auditions the single note/chord at that position once. This applies to mid-edit changes and pasting. It does not re-audition for movements within the same pronunciation unit or just redrawing. It reflects the previous tempo, octave, and note length, and does not produce sound at rest or command positions. Normal playback stops when editing is opened, and overall playback does not resume after the audition note ends. The audition note is also released upon the next pronunciation unit, cancellation, confirmation, or timbre exchange. Auditioning does not change the confirmed text or normal selection.
  - For loop playback of confirmed MML, the `Velocity` field setting takes precedence over any in-input specification. CC1 can also be controlled from the `CC1 modulation` field. If the performance event capacity is reached due to a very dense phrase, sounding notes are released, and playback stops. It can be restarted with `Stop` → `Play`.
  - Timbre changes from app Favorites/History, Browse patches, Random patch, or Random effect maintain the stop/play state. During playback, it starts from the beginning of the phrase and does not carry over mid-phrase waits from the previous phrase. Leading rests specified in MML are preserved. During the timbre browser, `Space` for play/stop and MML input start in the background are suppressed. Timbre changes via the plugin's own GUI are not detected.
  - For Six Sines/TyrellN6 CLAP, when audio processing is reconstructed due to loading, state restoration, or connection changes, it waits for the first phrase to start while continuing normal audio processing to preserve the initial sound. At 48kHz, the wait is approximately 21ms for Six Sines and 85ms for TyrellN6. Leading rests in MML are added further. See [ADR 0019](docs/adr/0019-random-patch-all-catalog.md) for application conditions and validation records.
  - In the `Velocity` field below the playback pattern, you can choose `100`, `127`, `40-100`, `80-127` using the same left/right arrows or dropdown. The initial value is 100. Range specification increases the value in order of Note On within the phrase, then decreases it in the next loop, repeating the action. Changes are applied from the next note without restarting the phrase and are restored on next launch.
  - In the `CC1 modulation` field, you can choose `0`, `127`, `sweep` using left/right arrows or dropdown. The initial value is 0. `sweep` repeats an action of changing from 0 to 127 in 2 seconds, then from 127 to 0 in the next 2 seconds. The selection is restored on next launch.
  - Guitar open strings send Note On for MIDI note numbers 40, 45, 50, 55, 59, 64 (E, A, D, G, B, E) at 125ms intervals. 2 seconds after the last Note On, all 6 notes are Note Off simultaneously, and it repeats after 0.5 seconds.
  - Upon successful `Load`, it is remembered when `Sequence` type is selected, started, or stopped. The memory remains after stopping or `Remove`. Old history/favorites are restored with the traditional 4-note pattern.
  - Upon exit and `Remove`, the binary state of timbre/parameters etc. exposed by the plugin via UAPMD's state API is saved per plugin to `%LOCALAPPDATA%\cat-plugin-player\states\`. On next launch or reloading the same plugin, it is restored before playback starts.
  - `Load` for an instrument replaces only the sound source, maintaining connected effects and settings. It saves the state of the plugin being replaced, then switches after successfully generating, restoring, and connecting the new plugin. If it fails, it retains the original configuration and displays an error.
  - The scope of state saving depends on the plugin and UAPMD's format-specific implementation. UI state is also requested, but the current UAPMD VST3 implementation only retrieves component state. It does not include duplicating external samples or saving upon abnormal termination.
  - On launch, it directly loads the previous instrument and effects, restores their state, and then starts playback. After that, it initializes the GUI and scans the list in the background.
  - If direct loading is not possible due to old history or file movement, it restores from the scan results after the GUI is displayed. Old history is updated once played.
  - If the main sound source is CLAP Floe, when audio processing is reconstructed due to loading or timbre restoration, the start of the first phrase is delayed by 100ms. Leading rests in MML are added further. This is a provisional measure for observing symptoms of missing initial sound, and resolution on actual hardware is unconfirmed. See [ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md) for application conditions and review policy.
  - Refer to [docs/startup-timing.md](docs/startup-timing.md) for startup time measurement methods and results.
- Effect Routing
  - Serial connection: 1 instrument → multiple effects → output. After loading an instrument, press `Load` for the first effect, then `Add` to append subsequent effects. Connected effects can be `Remove`d one by one. Duplicate connections of the same format + ID effect are not allowed.
  - `Random effect` next to `Sequence` randomly selects a preset from installed CLAP effects, using the same catalog and selection targets as cmrt's effect selector. If there are 0 effects, it adds the first one; if 1 effect, it replaces that effect; if multiple, it uniformly selects and replaces one of the connected effects. Repeated clicks do not increase the number of effects, and it maintains the order, settings, and Bypass state of other effects. To avoid duplicate connections, format + IDs already connected, other than the replacement target, are excluded from candidates. If no suitable candidate is available for the target slot, it displays a reason and does not change.
  - Random effect requires an instrument. Candidates are obtained from default effect/preset locations explored by the play-server shared core, and no substitution to VST3 is made. During preparation, current playback continues, and load/scan/timbre operations are disabled. If stopped, it remains stopped after application; if playing, it restarts the current phrase from the beginning, maintaining MML/Velocity/CC1 overall selection. On failure to generate/apply/connect, it reverts to the original configuration, and no automatic re-selection of other candidates occurs. Upon success, the applied state and connection are saved for next launch, and Favorites/History are not added. Save failures are displayed as applied.
  - `Routing` displays the connection order. You can change the order by dragging the `↕` button of an effect up or down. During drag, the moving effect name and insertion line are displayed. Instruments are not subject to reordering. You can edit timbres and effects with `Show UI` for each plugin. `Instrument` / `Effect` are displayed with the same type color as the list on the left.
  - Removing all effects reverts to a direct connection from the sound source to output. `Bypass effect` targets the entire chain, allowing passthrough while retaining instance and settings. It can also be toggled with the `b` key (does not work in MML input screen, Browse patches display, or during text input). During Bypass, connected effects in Routing are grayed out, with `Bypassed` appended.
  - `Sequence` is for the instrument. Stopping `Sequence` continues audio processing to allow effect decay.
  - On next launch, it restores the instrument, the order of all effects, their respective states, and Bypass status before starting playback. History for one old-format effect can also be loaded. If effect restoration fails, an error is displayed, and it reverts to a direct connection from the sound source, retaining the saved configuration.
  - Add/Remove/Reorder operations are saved only upon successful preparation of the new connection. If unsuccessful, it reverts to the original configuration and playback, and displays an error.
  - Supports mono/stereo for main input/output. Mono → stereo is duplication, stereo → mono is an average of left/right. Auxiliary inputs are silent.
  - Changing connections or Bypass may cause a brief audio dropout, and sounding notes and decay may be interrupted. Sidechain, parallel connections, and host-side Dry/Wet are not supported.
- Timbre
  - `Browse patches` near `Random patch` or the `T` key opens a maximized, independent timbre browser window on the same screen as the main app. It does not change the main window's position or size. It cannot be opened during MML editing. Filter by Role → Preset → Search with borders. The left condition area and right Patches list are individually scrollable. The list uses the remaining width and height, usable even when unmaximized or in a narrow window. The top row has Search, `Solo/Mute`, `Random`, `☰`. The search field uses the remaining width, and the `/` key in the browser moves input to the search field. Detailed sound source conditions can be seen by hovering `Solo/Mute`. Clicking this button or pressing the `M` key in the browser opens a vertically stacked Solo/Mute overlay, confirmed all at once with `OK` or `Enter`. Solo/Mute appears immediately to the right of the sound source name. Clicking the selected mode again reverts to normal; `Esc` / clicking outside the overlay discards changes. Multiple Solos are treated as OR, Mute as exclusion. Search uses case-insensitive regular expressions, space AND, `-condition` for exclusion, and `plugin:condition` (e.g., `pad -bright`, `plugin:floe`, `plugin:surgext -bass`). Aliases of integrated timbres are also searchable. Invalid regex results in an error, stopping application,抽選, and condition saving. If 0 results, review conditions by editing the search field or changing Role/Preset. Catalog loading wait/failure is displayed separately from 0 results. Role, Preset, and search string are saved on exit and restored on next launch. Preset is restored by name; if the condition is deleted, it reverts to `ALL` for that Role.
  - The list displays single-line timbre name cells that don't wrap, arranged in multiple columns (left → right → next row) depending on width. In a narrow window, it reverts to 1 column. Resizing maintains the selected timbre; changing only the number of columns does not reload. Clicking a timbre does not move the list's display position. Up/down/left/right keys or `h` / `j` / `k` / `l` move as per visual arrangement. Left/right move to previous/next timbre (or end of adjacent row at row ends); up/down move to rows above/below, and to the end of a short final row if no timbre directly below. Inputting a number first, like `2j` or `10k` (vim-style), moves that many times (stops at ends; numbers are canceled by non-movement keys). `PageUp` / `PageDown` move 10 times up/down, and combined with a number, move that multiple. Key movement, Random, filtering, and column count changes scroll only when the selected row is off-screen. It does not scroll if all timbres fit. For single sound source results, the sound source name is omitted; it's retained when mixed. Text color shows only one difference in results. If sound sources are mixed, the sound source name part is color-coded per sound source. For a single sound source, if Preset is `ALL`, timbre names are color-coded per relevant Preset; if that doesn't distinguish, then per category. Colors are Monokai-based. If all are the same, it's normal text color. Colors cycle through 5 hues, so distant different groups might have the same color. Pink timbres prioritize pink for their name. Classification folders are omitted; if distinguishing same-name timbres, necessary paths are retained. Dexed program assignment is maintained. Hovering displays the full display path, source application path, valid category, and merged count if 2+. Load/Size are not always present in the list; `no category` / `1 merged` are also not displayed. Normal timbres immediately request application when selection changes via mouse click/key move, and the same action occurs when a different candidate is chosen by condition change. Simply opening does not change the timbre. When opened, if the timbre playing from Browse patches / Random patch is in the current results, that timbre is selected and scrolled into view (not reapplied). The playing timbre is saved on exit and carried over as the restored timbre on next launch. Adding/deleting/Random effect maintains this carry-over. If the playing timbre is not in the results or unknown, it retains the selection from when it was last closed (unselected when opened for the first time after launch). After changing timbre via Favorites/History, or instrument load/delete, the playing timbre is unknown, and selection is cleared. Key movement starts from the selection when opened; if unselected, it moves to the top. Next selection is possible during preparation, and unstarted requests are replaced by the latest one. Upon successful normal application, duplicate timbre names and fixed margins at the bottom are not displayed. A brief message is displayed only if the selected and applied timbre differ, indicating that the previous timbre might continue playing for unconfirmed pinks. Pink's Space prompt and application failure/save error are displayed.
  - Pink timbres with sample capacity of 64,000,000 bytes or more are not automatically applied (unknown capacity is treated as normal). After selection, `Space` opens a confirmation overlay for sound source/timbre/capacity, and pressing `Enter` within it starts preparation/application. `Esc` during confirmation cancels without changing. Once applied, a timbre still needs re-confirmation. A progress overlay is shown during loading, and mid-process cancellation is not possible. `Space` / `Enter` in the search field, save menu, or Solo/Mute overlay are not used for timbre application ( `Enter` in Solo/Mute overlay is same as `OK`).
  - `Random` or the `R` key within the browser selects uniformly from the displayed results. During text input, `T` / `R` / `M` / `/` / `h` / `j` / `k` / `l` / numbers / `Enter` shortcuts do not work, and `R` / `M` do not work while the browser's overlay or save menu is displayed. For pink, only selection is needed, then the same Space→Enter confirmation. The external Random patch uniformly selects from all available patches as usual. `Esc` / `Enter` or OS close of the browser window closes only the browser, retaining the main app and the last successfully applied timbre, discarding unstarted requests and old preparation results. Processes that have already started applying will complete. When the confirmation overlay, Solo/Mute, or save menu is open, `Esc` / clicking outside the overlay only closes the child, not passing the same input to the browser or background operations. OS close closes the entire browser. It can be reopened, and there is no undo operation to revert to the original timbre.
  - Search conditions can be added to Role via `Save condition to Role` in the hamburger `☰` menu, and deleted via `Delete` in Preset. Adding from ALL goes to Etc. They are handled by common selector normalization/classification rules and saved to `patch-filter-presets.json` in the cat's config directory. Upon read corruption, the original file is protected, writing is stopped, and save failure is displayed on screen. Role/Preset/Search/Solo/Mute are maintained during the same app launch, but only saved custom conditions are restored after restart. TUI conditions and Favorites are not shared/migrated.
  - Browse patches uses the same catalog, supported CLAP instruments, state application, and failure recovery as Random patch. It maintains MML/Velocity/CC1 overall selection, and effect order/state/Bypass; upon success, it saves the state for next launch. Favorites/History are not automatically added. Successful application and save failure are displayed separately. Progress of preparation/application is updated even when child windows are focused.
  - `Random patch` in `Sequence` uniformly selects from all patches of Surge XT, Dexed, Floe, sforzando, Six Sines, TyrellN6, Vaporizer2 CLAP timbres in the existing cmrt catalog that match installed instruments and format + ID. It's not a uniform selection per sound source; Dexed, having more candidates, will be selected more frequently. The tooltip displays target sound sources and total candidate count. If no compatible candidates, the button is not displayed.
  - The Random patch catalog uses the existing `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` in read-only mode. Random patch does not substitute VST3, does not select effect timbres, and does not target unknown sound sources. For effect timbre selection, use Random effect in the effect routing section. Dexed retains program specification within cartridges. Floe/sforzando external samples are required at their original locations; there is no function to duplicate/save samples.
  - During Random patch preparation, current playback continues, and conflicting load/scan/timbre operations are disabled. There may be a wait for sample loading for Floe, etc. If stopped, it remains stopped after application; if playing, it restarts the current phrase from the beginning. MML/Velocity/CC1 overall selection and effect order/state/Bypass are maintained. Upon success, the applied state is saved for next launch, and Random patch itself does not add to Favorites/History.
  - If preparation/application fails, it displays the reason and does not automatically select another patch. It restores the original configuration/state/playback state, and instances where state restoration fails are prevented from saving. Save failure after timbre application is displayed as applied. Plugin-specific preparation/reflection conditions and validation scope are recorded in [ADR 0019](docs/adr/0019-random-patch-all-catalog.md).
  - `★ Add favorite` for each plugin allows saving multiple snapshots of the current timbre/settings as favorites. For sound sources, it also saves the playback pattern, stopped selection, and confirmed MML/chord text. For effects, it does not save playback settings. Names are auto-generated and can be changed later.
  - When loading another plugin, the timbre/settings/playback pattern of the plugin being replaced are automatically recorded to History before switching. The name is `Plugin Name Sequence Number`. It is not added for reloading the same plugin or restoring on startup. Confirmed MML/chord text and stopped selection for sound sources are also recorded within the same scope as favorites. For effects, playback settings are not recorded. If saving fails, the switch is aborted.
  - During manual/automatic saving, it checks for duplicates of the same plugin, role, playback pattern, confirmed MML text, and state, and reuses existing items and names. For automatic History recording, a difference only in stop state is treated as identical, but MML with different text is kept as a separate item. Effects do not compare playback settings.
  - [Plugin-Specific Processing Policy/List (ADR 0002)](docs/adr/0002-plugin-specific-favorite-state-comparison.md) provides exceptions, reasons, scope, and limitations for each product. For TyrellN6 CLAP's verified state format, compressed parts that change only with playback are excluded from duplicate determination, and timbres with identical text timbre settings are grouped. Verified PCore `UI_op=9/10` and trailing blank lines in text are also excluded from comparison ([ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)). For Vaporizer2 CLAP's verified format, minute differences in MSEG time/coordinates that occur during loading/saving (absolute difference for specified items: time `0.00011` ms or less, normalized coordinates `0.000001` or less. See [ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md) for details) are treated as identical. Edits within this range cannot be distinguished. Saved data is retained as is.
  - Switch lists with the `Favorites` / `History` / `Plugins` tabs in the center. Favorites maintain manual order; new additions are placed at the top. Calling, re-saving, or recording to History does not change the order. Can be filtered by favorite name, plugin name, and format.
  - In Favorites' hamburger menu, turning ON `Reorder favorites` allows reordering with `↑` / `↓` at the end of the row, saving immediately. Row click loading and Effect removal work as before. `Sort by plugin name` sorts once in ascending order by plugin name and saves (case-insensitive, preserving relative order for same-name items). Manual adjustments are still possible afterward. Sorting is disabled during filtering, and reorder mode is OFF on startup. See [Decision Background and Specification (ADR 0017)](docs/adr/0017-manual-favorites-order.md).
  - History is in order of most recent registration. If a timbre is determined to be the same, the existing timbre is reused, its registration time is updated, and it moves to the top. Elapsed time is displayed in units of `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` (month is 30 days, year is 365 days). Manual Favorites are retained. All old-format items are migrated to History, and items not starting with `auto ` are also kept in Favorites (because old formats did not record the origin of automatic saves, all renamed automatic saves are included in History). See [Policy and Reasons (ADR 0016)](docs/adr/0016-history-instead-of-automatic-favorites.md).
  - History does not display Rename/Delete operation menus. Renaming/deleting is done in Favorites.
  - `Show only CLAP for duplicate plugins` in the global settings `☰` is ON by default. If there are CLAP plugins with the same name, manufacturer, and type, VST3s in the Plugins list are hidden (ignoring leading/trailing spaces and case differences in name/manufacturer; if name or manufacturer is empty, both formats are shown). Turning OFF shows both formats. The setting is saved to `[plugins] prefer_clap` in `config.toml` and maintained on next launch. It does not change Favorites or the format loaded/restored.
  - On both tabs, clicking the name/image loads; for connected effects, clicking again removes them. The currently loaded/connected row is highlighted, and the color is maintained even when an effect is bypassed. `Load` / `Add` / `Remove` buttons in Plugins perform the same operations.
  - Favorites are displayed one item per line. Long names/supplementary info are truncated; hover to see the full text. Waveform/spectrum is displayed below Routing on the right; the right panel width can be adjusted by dragging the boundary. `Audio analysis`'s `On right` toggles analysis display to bottom right / bottom of screen (bottom right on startup).
  - For sforzando CLAP's verified state format, it doesn't add identical timbres based only on save counter and differences in note/CC1 playback values, but reuses existing favorites. Differences in timbre settings are preserved. See [ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md) for scope and limitations.
  - A colored `Instrument` / `Effect` is displayed to the left of each row. Clicking a favorite name restores its timbre/settings. For instrument favorites, it restores the saved confirmed text and playback pattern selection. If stopped, it remains stopped; `Play` plays the restored phrase. If playing, it switches to the saved pattern/MML. Favorites in old format without text retain the current text; old-format stopped items retain the current selection. If conversion or state load/connection fails, it restores the original text/selection/playback state and does not save the failed content to the session. Instrument favorites maintain connected effects; effect favorites maintain the instrument and MML. An instrument is required to recall an effect.
  - Connected effect favorites display `Connected - click again to remove`, and re-clicking the name removes that effect. This also applies if settings were edited after recall or if bypassed. The instrument, other effects, and saved favorites remain. Another timbre of the same format + ID switches settings at the current position, and unconnected effects are added to the end. The selected favorite for each effect is also restored on next launch.
  - Rename/Delete operations are available from `…` at the end of the row. Edits after recall or conventional automatic saves do not overwrite favorites. `Last favorite` indicates the last saved/recalled favorite.
  - Favorites are saved to `%LOCALAPPDATA%\cat-plugin-player\favorites\`. Short audio dropouts may occur during saving/restoring, and the savable scope is the same as conventional state saving.
- Lissajous Display
  - `Lissajous` is displayed below the oscilloscope in the right-side analysis panel. If `☰` → `On right` is unchecked for the oscilloscope, spectrum, oscilloscope, and Lissajous align horizontally at the bottom of the screen.
  - A 45° rotated Lissajous curve draws the most recent 50ms output (max 4096 samples), reflecting effects and Bypass. A common automatic scaling preserves volume differences and aspect ratio of the shape; in-phase signals at similar levels become a vertical line, out-of-phase become a horizontal line, and sine waves with phase differences become ellipses or circles. Left-only output appears as a diagonal line from top-left to bottom-right, right-only from top-right to bottom-left, mono as a vertical line, and silence as a central dot.
  - It updates independently of notes, cycle counts, or trigger settings, and `Show analysis labels` can display headings. Axes and drawing method can also be checked in the graph's tooltip.
  - `Correlation` directly below Lissajous displays the left/right correlation for the most recent 150ms as a bar and numerical value from -1 to +1. +1 is in-phase, near 0 indicates weak correlation, and -1 is out-of-phase. Negative values are shown in red, serving as a guideline for cancellations when mixed to mono. It shows "—" if silent or one side is silent.
- Waveform Display
  - The `Oscilloscope` at the bottom of the screen displays the L (green) and R (blue) output, reflecting effects and Bypass, superimposed. Mono displays the same waveform on both.
  - The display width is determined from the MIDI note number of the last Note On (A4 = 440Hz), and `1`, `2`, `4`, `8 cycles` can be selected. After Note Off, the decay is displayed based on the last note. Output containing chords or detuning may not repeat perfectly at that width.
  - `Zero cross` aligns with the negative → positive crossing of the left channel. If the left channel is silent, it uses the right channel; if no crossing, it displays from the beginning.
  - `Similarity` comprehensively searches for the position that maximizes correlation with the previous display, over the display width, 1 sample at a time, from the start of the buffer. In case of a tie, the earlier position is adopted, and the same start position is used for left/right. For the first display or if the comparison target is silent, zero cross is used.
  - The comparison history is reset when a note sounds, playback pattern changes, display cycle count changes, trigger method changes, or the audio stream changes. A waiting display appears until sufficient data is collected.
  - Exploration is performed on a separate thread from audio processing. Display updates may be slow for low notes or high cycle counts. If display data is interrupted, the history is reset.
  - The default display settings on startup are 4 cycles and zero cross. Display settings are not saved.

# Build & Test Procedure
- *Prerequisite: First, build [UAPMD](https://github.com/atsushieno/uapmd). Refer to its documentation for steps. If you have `Microsoft C++ Build Tools` on Windows, it's easy! If not, ask AI to install it, and it's easy! The overall image is: "It's easy if you let AI handle everything!"*
- *Next prerequisite: If Rust is not installed, ask AI to install it, and it's easy!*
- As mentioned, ensure `../uapmd` is built.
- Confirm that audio plugins (instruments) like Surge XT are installed. For example, confirm Surge XT plays in REAPER.
- Run `cargo run` (the app will be debug-built).
- Confirm that the screen opens.
- Confirm that Surge XT etc. are listed on the screen.
- Click the Load button.
- Confirm that it loads and plays sound.

# Miscellaneous

This text was generated by AI and is hard to read. I plan to revise it later.

## Window Position/Size Settings

You can specify the main window's startup position/size and plugin GUI placement method in `%LOCALAPPDATA%\cat-plugin-player\config.toml`. Please edit it after closing the app. Session and favorite selection states are saved to `status.json` in the same directory; normal saving does not overwrite the TOML. If they are mixed in the old TOML, they are migrated on first launch, the original file is saved as `config.toml.before-status-migration.bak`, and only state items are removed. Comments, formatting, and unknown items in settings are retained, and independent comments attached to deleted items are left at the end. If an existing `status.json` is present, it takes precedence.

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

The main window's `x` and `y` are logical coordinates and are used as the startup position if both are specified. If omitted, it uses the conventional initial position. `width` and `height` are the inner width and height (logical coordinates) of the main window. You can specify only one, and omitted sides or values <= 0 / non-finite revert to width 900, height 600. Changes will be reflected on next launch. The plugin placement method is currently only `right_then_bottom_right`, and the behavior is the same if omitted. GUI size and placement are calculated based on the actual window frame, screen work area, and DPI. Automatic saving of positions moved manually, or per-plugin position specification, is not performed.

## Local Build and Execution

Clone this app, `uapmd`, `clap-mml-render-tui`, and `clap-mml-play-server` into the same parent directory, then execute from this app's directory. TUI's `app` / `patches` / `patch-select` / `tui-core` and play-server's `core-lib` / `server-config` always use local references from Cargo.toml. Play-server needs `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin` in addition to conventional `prepare_clap_patch_state` / `PatchStateError`, and TUI's `cmrt-patches` needs to re-export them. The timbre browser uses public Role/Preset/filter APIs and tui-core's sample capacity check. Random effect uses play-server's `AudioEffectCatalog` / `EffectRenderer` and TUI's effect selector's selection conditions. Missing checkouts or insufficient APIs will result in build errors, and no automatic switch to Git versions will occur.

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

If UAPMD is not in the sibling `uapmd` directory, first set `$env:UAPMD_DIR = 'X:\projects\uapmd'`. The location of UAPMD during build is specified by an environment variable. `config.toml` is not read as build settings.

Do not use `cargo install`; directly execute the `exe` generated in the default `target/release`. The build uses local code (including uncommitted changes) and existing `target/shim`, but source changes may require reconstruction/rebuilding. Shim DLLs are bundled with the exe, extracted to the cache directory and loaded when the GUI starts.

Refer to [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) for the 3-repo dependency structure and operational policy. Local switching of TUI is not needed for building with cat. If building TUI itself across repositories, use TUI's existing `python scripts/cross_repo_local.py on` procedure, and `off` is handled manually. Local references use uncommitted changes, and Cargo.lock does not fix the content of each checkout. When verifying, record the HEAD and diff of each repo.

Launches the GUI with no arguments. `--help` and `--version` are also available.

# Future Brainstorming
- *May change based on mood.*
- Enhance the strengths not present in cmrt, such as VST3 and GUI support.
  - Automatic GUI operation?
    - Log GUI operations and reproduce them swiftly with TUI in cmrt? Provide hints for plugin-specific customization?
  - Experiment with egui.
    - Keyboard display?
    - Grid sequencer?
- Utilize various cmrt crates.
  - *This will also refine the crates, and we benefit from using them, so it's a win-win.*
  - Patch selector
    - Can detect preset patches for CLAP audio plugins.
  - CLI import / export
    - The cmrt keyboard screen can be triggered from the CLI, specifying patch, effect, arpeggio, etc., to play sound immediately. Enable the same for this app (to the level where it works by just changing cmrt to cat-plugin-player).
      - Frequent destructive changes are acceptable. Manual migration work is possible later. It's better than being paralyzed by fear of change and halting progress.

# Casual Thoughts Corner
- *Related to concepts.*
- I love audio plugin presets.
  - Because they are reproducible.
  - Because they are shared freely, benefiting everyone.
- I want to provide a musical instrument UX.
  - Not a DAW UX, but a musical instrument UX.
- What is a musical instrument?
  - It's a combination of all layers, from the audio plugin to the PC keyboard.
    - There are intermediate layers.
      - These are plugin hosts and performance UIs.
        - This is a layer that can be "vibe coded".
        - Building this can improve the UX.
- For whom is this instrument?
  - Cats, babies.
    - This means expanding the target range to the beginner side, including cats and babies (e.g., Android tablets).
  - *This is purely a metaphor. Actual operation on Android tablets is unconfirmed. The likelihood of efforts to make it work on Android if it doesn't is also low.*

# Out of Scope, Not Aimed For
- Robustness
  - Absolutely no bugs, never crashes.
  - Full compatibility. Loads and operates with all past settings and data.
- Features
  - Equipped with every possible feature. Usable for all purposes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed with no CPU load in any environment. Audio never cuts out.
- Demands
  - Immediate response to every user request.

# License
- MIT