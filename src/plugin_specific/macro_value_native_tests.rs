//! Macro values moved through host parameters keep the saved favorite the same.
//! Run each case alone (`--exact`): a second native host in one process never finishes opening.
use crate::{
    config::PluginKey,
    favorites_store::Library,
    ffi::{
        self,
        parameters::{AutomationEvent, Parameter},
    },
    instrument_macros::Instrument,
};
use std::ffi::{c_char, c_void};

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    unsafe {
        *(user as *mut Option<Result<i32, String>>) =
            Some(ffi::error_string(error).map_or(Ok(id), Err));
    }
}

pub(super) const BLOCK: usize = 1024;

/// Floe ignores parameter events for a while after its state loads unless real time passes.
pub(super) fn wait_one_block() {
    std::thread::sleep(std::time::Duration::from_secs_f64(BLOCK as f64 / 48000.0));
}

/// Prepares a processor, sends the parameters `pick` chooses from fresh bindings at the
/// start of about `blocks` × 21 ms, and returns them with their values read back.
fn render(
    host: &ffi::Host,
    id: i32,
    pick: impl Fn(&[Parameter]) -> Vec<&Parameter>,
    blocks: usize,
) -> Vec<(String, f64, f64)> {
    let processor = host
        .create_chain(id, &[], false, 48000, BLOCK as u32)
        .unwrap();
    processor.prepare_parameters().unwrap();
    let parameters = host.parameters(id).unwrap();
    let events: Vec<_> = pick(&parameters).into_iter().map(moved).collect();
    let mut audio = unsafe { processor.audio_ref() };
    let mut output = [0.0; BLOCK * 2];
    for block in 0..blocks {
        let events = if block == 0 { &events[..] } else { &[] };
        audio.process_automation(events, &mut output, 2).unwrap();
        host.pump();
        wait_one_block();
    }
    let after = host.parameters(id).unwrap();
    drop(processor);
    events
        .iter()
        .map(|e| {
            let sent = parameters
                .iter()
                .find(|p| p.binding == e.parameter.with_value(p.binding.value()))
                .unwrap();
            let now = after
                .iter()
                .find(|a| a.stable_id == sent.stable_id)
                .unwrap();
            (sent.name.clone(), e.parameter.value(), now.current)
        })
        .collect()
}

/// Moves `parameter` most of the way to the far end of its range.
fn moved(parameter: &Parameter) -> AutomationEvent {
    let (low, high) = (parameter.minimum, parameter.maximum);
    let far = if parameter.current - low > high - parameter.current {
        low + (high - low) * 0.1
    } else {
        high - (high - low) * 0.1
    };
    AutomationEvent::parameter(parameter.binding.with_value(far))
}

/// The first saved favorite of the CLAP `id_of_plugin` whose state routes a macro:
/// (its name, plugin, state).
pub(super) fn routing_favorite(id_of_plugin: &str) -> (String, PluginKey, Vec<u8>) {
    let config = crate::config::path().unwrap();
    let saved: Library = toml::from_str(
        &std::fs::read_to_string(config.parent().unwrap().join("favorites/index.toml")).unwrap(),
    )
    .unwrap();
    let instrument = Instrument::of("CLAP", id_of_plugin).unwrap();
    saved
        .entries
        .iter()
        .filter(|e| e.plugin.format == "CLAP" && e.plugin.id == id_of_plugin)
        .find_map(|e| {
            let state = saved.state(&config, &e.id).ok()?;
            let macros = instrument.routed("CLAP", &state)?;
            (!macros.is_empty()).then(|| (e.name.clone(), e.plugin.clone(), state))
        })
        .expect("a saved favorite routing a macro")
}

/// A host with one instance of `plugin`; the state is not loaded yet.
pub(super) fn open(plugin: &PluginKey, sample_rate: u32) -> (ffi::Host, i32) {
    let host = unsafe { ffi::Host::new(wake, std::ptr::null_mut()) }.unwrap();
    let info = host.restore_plugin(plugin).unwrap();
    let mut result: Option<Result<i32, String>> = None;
    unsafe {
        host.create_instance(
            info.index,
            sample_rate,
            BLOCK as u32,
            created,
            &mut result as *mut _ as *mut c_void,
        )
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while result.is_none() {
        assert!(std::time::Instant::now() < deadline);
        host.pump_startup();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let id = result.unwrap().unwrap();
    (host, id)
}

fn check(id_of_plugin: &str) {
    let _dll = crate::native_library::load().unwrap();
    let instrument = Instrument::of("CLAP", id_of_plugin).unwrap();
    let (name, plugin, state) = routing_favorite(id_of_plugin);
    let (host, id) = open(&plugin, 48000);
    host.load_state(id, &state).unwrap();
    render(&host, id, |_| vec![], 50);
    let settled = host.save_state(id).unwrap();
    let positions = |state: &[u8]| -> Vec<_> {
        instrument
            .routed("CLAP", state)
            .unwrap()
            .iter()
            .map(|m| (m.number, m.position))
            .collect()
    };
    let is_macro = |p: &Parameter| (1..=8).any(|n| instrument.parameter(n, &p.name));
    let sent = render(
        &host,
        id,
        |all| all.iter().filter(|p| p.eligible && is_macro(p)).collect(),
        20,
    );
    let played = host.save_state(id).unwrap();
    println!("{}: {sent:?}", name);
    println!("{:?} -> {:?}", positions(&settled), positions(&played));
    assert!(!sent.is_empty());
    for (name, value, current) in &sent {
        assert!(
            (value - current).abs() < 1e-3,
            "{name}: {value} != {current}"
        );
    }
    assert_ne!(
        positions(&played),
        positions(&settled),
        "the macro values must reach the state"
    );
    assert!(super::same_favorite_state(&plugin, &state, &settled));
    assert!(super::same_favorite_state(&plugin, &settled, &played));

    // A sound parameter next to the macros still makes a different favorite.
    let sound = render(
        &host,
        id,
        |all| {
            all.iter()
                .filter(|p| p.eligible && p.maximum > p.minimum && !is_macro(p))
                .take(1)
                .collect()
        },
        20,
    );
    let edited = host.save_state(id).unwrap();
    println!("edited {sound:?}");
    assert!(!super::same_favorite_state(&plugin, &played, &edited));
    host.destroy_instance(id);
}

#[test]
#[ignore = "requires installed Vaporizer2 CLAP and saved favorites (read only)"]
fn native_vaporizer2_macro_values_keep_the_favorite() {
    check("com.vastdynamics.VAST2");
}

#[test]
#[ignore = "requires installed Six Sines CLAP and saved favorites (read only)"]
fn native_six_sines_macro_values_keep_the_favorite() {
    check("org.baconpaul.six-sines");
}

#[test]
#[ignore = "requires installed Floe CLAP and saved favorites (read only)"]
fn native_floe_macro_values_keep_the_favorite() {
    check("com.floe-audio.floe");
}
