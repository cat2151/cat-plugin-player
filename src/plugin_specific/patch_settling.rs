//! Sample-based note withholding for the measured Six Sines/Tyrell state queues.
use crate::config::PluginKey;

pub(super) fn delay(plugin: &PluginKey, sample_rate: u32) -> std::time::Duration {
    if plugin.format != "CLAP" {
        return std::time::Duration::ZERO;
    }
    // The audio callback caps every process block at MAX_BLOCK_FRAMES. Four
    // blocks and 4096 frames cover Tyrell's existing cmrt loader contract; one
    // entirely empty block installs Six Sines state without losing its note.
    let frames = match plugin.id.as_str() {
        "org.baconpaul.six-sines" => crate::audio::MAX_BLOCK_FRAMES,
        "com.u-he.TyrellN6" => 4096u32.max(4 * crate::audio::MAX_BLOCK_FRAMES),
        _ => 0,
    };
    std::time::Duration::from_secs_f64(f64::from(frames) / f64::from(sample_rate.max(1)))
}

#[cfg(test)]
mod tests;
