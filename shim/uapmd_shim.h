/* uapmd_shim.h - minimal C ABI over uapmd-plugin-hosting, for use from Rust.
 *
 * Threading rules:
 *  - Every function must be called on the main (UI) thread.
 *  - UhScanDoneFn / UhInstanceFn are invoked on the main thread, from inside uh_pump().
 *  - UhWakeFn may be invoked from ANY thread. It must only ask the UI to call uh_pump() soon.
 *  - Call uh_pump() regularly (every UI frame). Worker threads inside uapmd block
 *    until the main thread pumps, so a UI that stops pumping stalls them.
 */
#ifndef UAPMD_SHIM_H
#define UAPMD_SHIM_H

#include <stdint.h>

#if defined(_WIN32)
#  if defined(UAPMD_SHIM_BUILD)
#    define UH_API __declspec(dllexport)
#  else
#    define UH_API __declspec(dllimport)
#  endif
#else
#  define UH_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct UhHost UhHost;

typedef void (*UhWakeFn)(void* user);
/* error is NULL on success. The pointer is valid only during the call. */
typedef void (*UhScanDoneFn)(void* user, const char* error);
/* instance_id is negative on failure. error is NULL on success. */
typedef void (*UhInstanceFn)(void* user, int32_t instance_id, const char* error);

/* Fields for uh_plugin_info(). */
enum {
    UH_PLUGIN_NAME   = 0,
    UH_PLUGIN_VENDOR = 1,
    UH_PLUGIN_FORMAT = 2,
    UH_PLUGIN_ID     = 3,
    UH_PLUGIN_PATH   = 4
};

/* Return codes for uh_ui_show(). */
enum {
    UH_OK              = 0,
    UH_ERR_NO_INSTANCE = -1,
    UH_ERR_NO_UI       = -2,
    UH_ERR_CREATE_UI   = -3,
    UH_ERR_SHOW_UI     = -4,
    UH_ERR_EXCEPTION   = -5
};

/* Only one host may exist per process. Returns NULL on failure. */
UH_API UhHost* uh_create(UhWakeFn wake, void* wake_user);
/* Blocks until a running scan has finished. */
UH_API void uh_destroy(UhHost* host);

/* Runs the tasks uapmd queued for the main thread, then delivers callbacks. */
UH_API void uh_pump(UhHost* host);
/* Before the GUI event loop starts: also dispatch native Windows messages. */
UH_API void uh_pump_startup(UhHost* host);

/* Scans on a worker thread. rescan == 0 uses the plugin list cache when there is one.
 * Returns 0 if started, -1 if a scan is already running. */
UH_API int32_t uh_scan_async(UhHost* host, int32_t rescan, UhScanDoneFn done, void* user);

/* The plugin list is a snapshot taken when the last scan finished. */
UH_API int32_t uh_plugin_count(UhHost* host);
/* Copies a UTF-8 string (NUL-terminated, truncated to buf_len) into buf.
 * Returns the full length in bytes without the NUL, or -1 for a bad index/field.
 * Call with buf == NULL to query the length. */
UH_API int32_t uh_plugin_info(UhHost* host, int32_t index, int32_t field, char* buf, int32_t buf_len);

/* Adds one saved entry without scanning. All strings are UTF-8. Returns its
 * snapshot index, or -1 if the bundle is missing or a scan is active. */
UH_API int32_t uh_restore_plugin(UhHost* host, const char* format, const char* id,
                                 const char* name, const char* vendor, const char* path);

/* Instantiates plugin #index of the snapshot. The result arrives through `done`. */
UH_API void uh_instance_create(UhHost* host, int32_t index, uint32_t sample_rate,
                               uint32_t buffer_size, UhInstanceFn done, void* user);
UH_API void uh_instance_destroy(UhHost* host, int32_t instance_id);

/* ---- Audio ----
 * A processor feeds one plugin instance with events and pulls audio out of it.
 *  - create/destroy: main thread.
 *  - process: the audio thread, and only one thread at a time.
 *  - Stop the audio stream BEFORE destroying the processor, and destroy the
 *    processor BEFORE destroying its instance or the host.
 */
typedef struct UhProcessor UhProcessor;

/* max_frames: the largest frame count that will ever be passed to process.
 * Returns NULL on failure. */
UH_API UhProcessor* uh_processor_create(UhHost* host, int32_t instance_id,
                                        uint32_t sample_rate, uint32_t max_frames);
UH_API void uh_processor_destroy(UhProcessor* processor);

/* Renders `frames` frames into out_interleaved (frames * out_channels floats).
 * ump_words: UMP (MIDI 1.0 or 2.0 channel voice messages) to deliver in this block;
 * may be NULL when ump_word_count is 0. Events start at sample 0 of the block; a JR
 * Timestamp UMP (0x0020nnnn) before an event moves it nnnn samples later.
 * Returns 0 on success; on failure the output is silence. */
UH_API int32_t uh_processor_process(UhProcessor* processor,
                                    const uint32_t* ump_words, int32_t ump_word_count,
                                    float* out_interleaved, int32_t out_channels, int32_t frames);

/* Shows the plugin editor in its own top-level window. Returns UH_OK or UH_ERR_*. */
UH_API int32_t uh_ui_show(UhHost* host, int32_t instance_id);
UH_API void uh_ui_hide(UhHost* host, int32_t instance_id);

#ifdef __cplusplus
}
#endif

#endif /* UAPMD_SHIM_H */
