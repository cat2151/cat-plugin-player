//! The shared state-queue settling contract, with the caller's stream stopped.
use super::{Host, Processor};

impl Host {
    /// Run on the main thread, with no other renderer or state operation active.
    /// Native processing takes its audio role inside the shim; no notes are sent.
    pub fn settle_processor(&self, processor: &Processor, rate: u32) -> Result<(), String> {
        let mut render = unsafe { processor.audio_ref() };
        let mut silent = [0.0; 2048];
        for _ in 0..rate.div_ceil(10240).max(8) {
            self.pump();
            if !render.process(&[], &mut silent, 2) {
                return Err("Plugin failed while settling selected state".into());
            }
        }
        Ok(())
    }
}
