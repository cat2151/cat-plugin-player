//! Main-thread ownership of restored instances. No app, GUI or MML dependencies.
use crate::{
    chain_settings::{PluginState, Settings},
    host::{self, Host, PluginKey, Processor},
};
use std::{
    ffi::{c_char, c_void},
    sync::mpsc,
    time::{Duration, Instant},
};

pub struct Chain {
    processor: Option<Processor>,
    ids: Vec<i32>,
    host: Host,
}

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let sender = unsafe { Box::from_raw(user as *mut mpsc::Sender<Result<i32, String>>) };
    let result = unsafe { host::error_string(error) }.map_or_else(
        || {
            if id >= 0 {
                Ok(id)
            } else {
                Err("Plugin creation failed".into())
            }
        },
        Err,
    );
    let _ = sender.send(result);
}

#[path = "default_instrument.rs"]
mod default_instrument;
pub use default_instrument::default_instrument;

impl Chain {
    pub fn open(rate: u32, settings: &Settings) -> Result<Self, String> {
        settings.validate()?;
        Self::open_inner(rate, settings, false)
    }

    #[allow(dead_code)] // Used by the editor and shared native contract tests.
    pub fn open_master(
        rate: u32,
        settings: &crate::master_effects::Settings,
    ) -> Result<Self, String> {
        settings.validate()?;
        Self::open_inner(
            rate,
            &Settings {
                effects: settings.effects.clone(),
                ..Default::default()
            },
            true,
        )
    }

    fn open_inner(rate: u32, settings: &Settings, master: bool) -> Result<Self, String> {
        let host =
            unsafe { Host::new(wake, std::ptr::null_mut()) }.ok_or("Cannot create plugin host")?;
        let mut chain = Self {
            processor: None,
            ids: Vec::new(),
            host,
        };
        if !master {
            let fallback;
            let key = if let Some(slot) = &settings.instrument {
                &slot.plugin
            } else {
                fallback = default_instrument()?;
                &fallback
            };
            let id = chain.create(key, rate)?;
            if let Some(slot) = &settings.instrument {
                chain.host.load_state(id, &slot.state)?;
            }
        }
        for slot in &settings.effects {
            let id = chain.create(&slot.plugin, rate)?;
            chain.host.load_state(id, &slot.state)?;
        }
        chain.processor = Some(
            chain
                .host
                .create_chain(
                    if master { -1 } else { chain.ids[0] },
                    if master { &chain.ids } else { &chain.ids[1..] },
                    false,
                    rate,
                    1024,
                )
                .ok_or("Cannot connect plugin chain")?,
        );
        chain.set_bypass(settings)?;
        chain
            .host
            .settle_processor(chain.processor.as_ref().unwrap(), rate)?;
        Ok(chain)
    }

    /// Applies each effect's bypass, or the whole chain's, without reopening.
    pub fn set_bypass(&self, settings: &Settings) -> Result<(), String> {
        let processor = self.processor.as_ref().ok_or("Chain has no processor")?;
        for (index, slot) in settings.effects.iter().enumerate() {
            processor.set_bypassed(index, settings.bypass || slot.bypass)?;
        }
        Ok(())
    }

    /// `set_bypass` for a chain from `open_master`.
    #[allow(dead_code)] // Used by the editor.
    pub fn set_master_bypass(&self, effects: &[PluginState]) -> Result<(), String> {
        self.set_bypass(&Settings {
            effects: effects.to_vec(),
            ..Default::default()
        })
    }

    /// Applies `settings` to the existing instances without recreating them.
    /// The caller checks `Settings::same_plugins` against the settings this chain was opened
    /// with, and stops every renderer first: the processor and its audio refs are replaced.
    #[allow(dead_code)] // Used by the editor and shared native contract tests.
    pub fn reload(mut self, rate: u32, settings: &Settings) -> Result<Self, String> {
        settings.validate()?;
        let slots: Vec<_> = settings
            .instrument
            .iter()
            .chain(&settings.effects)
            .collect();
        if settings.instrument.is_none() || slots.len() != self.ids.len() {
            return Err("Chain does not match the reloaded plugins".into());
        }
        self.processor.take();
        for (&id, slot) in self.ids.iter().zip(slots) {
            self.host.load_state(id, &slot.state)?;
        }
        self.processor = Some(
            self.host
                .create_chain(self.ids[0], &self.ids[1..], false, rate, 1024)
                .ok_or("Cannot connect plugin chain")?,
        );
        self.set_bypass(settings)?;
        self.host
            .settle_processor(self.processor.as_ref().unwrap(), rate)?;
        Ok(self)
    }

    fn create(&mut self, key: &PluginKey, rate: u32) -> Result<i32, String> {
        let plugin = self
            .host
            .restore_plugin(key)
            .ok_or_else(|| format!("Plugin not found: {} [{}]", key.id, key.format))?;
        let (tx, rx) = mpsc::channel::<Result<i32, String>>();
        unsafe {
            self.host.create_instance(
                plugin.index,
                rate,
                1024,
                created,
                Box::into_raw(Box::new(tx)) as *mut c_void,
            )
        };
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            // pump_startup would dispatch winit's wake-up inside an eframe frame,
            // and the app would then sleep until some other window message arrives.
            self.host.pump();
            match rx.try_recv() {
                Ok(result) => {
                    let id = result?;
                    self.ids.push(id);
                    return Ok(id);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("Plugin creation callback disconnected".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
            if Instant::now() >= deadline {
                return Err("Plugin creation timed out".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn pump(&self) {
        self.host.pump();
    }

    /// # Safety
    /// The callback must finish before this chain is destroyed, and only one thread may render.
    pub unsafe fn audio_ref(&self) -> host::ProcessorRef {
        unsafe { self.processor.as_ref().unwrap().audio_ref() }
    }
}

impl Drop for Chain {
    fn drop(&mut self) {
        self.processor.take();
        for id in self.ids.drain(..).rev() {
            self.host.destroy_instance(id);
        }
    }
}
