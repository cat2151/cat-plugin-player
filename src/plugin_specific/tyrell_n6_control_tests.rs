use super::*;
use crate::sequence_modulation::{cc, MACROS};

fn plugin() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "com.u-he.TyrellN6".into(),
        name: "TyrellN6".into(),
        vendor: "u-he".into(),
        bundle_path: String::new(),
    }
}

const SOURCE_NAMES: &str = "#ms=none\n#ms=ModWhl\n#ms=PitchW\n#ms=CtrlA\n#ms=CtrlB\n#ms=Gate\n";

/// A factory-preset layout: metadata, `#ms=` names, sections, then the compressed data.
fn state(usage: &str, routing: &str, assignments: &str) -> Vec<u8> {
    format!(
        "#pgm=Test.h2p\n/*@Meta\n\nUsage:\n'{usage}'\n\n*/\n\n#AM=TyrellN6\n#Vers=10010\n\
         {SOURCE_NAMES}#cm=PCore\nMMS1=1\nMMS2=0\n{assignments}#cm=Tyrell\nTMSrc1=0\n{routing}\n\n\
         // Section for ugly compressed binary Data\n// DON'T TOUCH THIS\n\n$$$$9\n?CtrlA=1\n=1\n\0\0"
    )
    .into_bytes()
}

const CCS: &str = "CtrlA=2\nCtrlB=11\n";

#[test]
fn each_used_control_reports_its_assigned_cc() {
    let both = controls(&plugin(), &state("", "PWSrc=3\nARMod=4", CCS));
    let mut expected = [None; MACROS];
    expected[..2].copy_from_slice(&[Some(2), Some(11)]);
    assert_eq!(both.ccs(), expected);
    assert_eq!(&both.labels[..2], ["Control A", "Control B"]);
    for (section, key) in [
        ("PCore", "MMVS2"),
        ("Tyrell", "FMSrc2"),
        ("LFO1", "FMS1"),
        ("LFO2", "DMS1"),
    ] {
        let routing = format!("#cm={section}\n{key}=3");
        let only_a = controls(&plugin(), &state("", &routing, CCS));
        assert_eq!(only_a.ccs()[..2], [Some(2), None], "{section}:{key}");
    }
}

#[test]
fn unused_unassigned_and_other_plugins_are_hidden() {
    let hidden = [None; MACROS];
    // ModWhl, a non-source key holding 3, and the same key in another section.
    for routing in ["PWSrc=1", "Sync=3", "#cm=LFO1\nPWSrc=3"] {
        assert_eq!(controls(&plugin(), &state("", routing, CCS)).ccs(), hidden);
    }
    assert_eq!(controls(&plugin(), &state("", "PWSrc=3", "")).ccs(), hidden);
    let out_of_range = "CtrlA=128\nCtrlB=11\n";
    assert_eq!(
        controls(&plugin(), &state("", "PWSrc=3", out_of_range)).ccs(),
        hidden
    );
    let other = PluginKey {
        format: "VST3".into(),
        ..plugin()
    };
    assert_eq!(controls(&other, &state("", "PWSrc=3", CCS)).ccs(), hidden);
    let unknown = String::from_utf8(state("", "PWSrc=3", CCS))
        .unwrap()
        .replace("#AM=TyrellN6", "#AM=Diva");
    assert_eq!(controls(&plugin(), unknown.as_bytes()).ccs(), hidden);
}

#[test]
fn usage_names_the_controls() {
    let usage = r"MW = lo pass\r\nCtrl A = bass dominator\r\nCtrl B =  short amp env ";
    let named = controls(&plugin(), &state(usage, "PWSrc=3\nARMod=4", CCS));
    assert_eq!(named.names[0].as_deref(), Some("bass dominator"));
    assert_eq!(named.names[1].as_deref(), Some("short amp env"));
    let unnamed = controls(
        &plugin(),
        &state(r"MW = lo pass\r\nCtrl A =", "PWSrc=3", CCS),
    );
    assert_eq!(unnamed.names[..2], [None, None]);
}

fn level(render: &mut crate::ffi::ProcessorRef, events: &[u32]) -> f64 {
    let mut output = [0.0f32; 512];
    render.process(&[0x2080_3000], &mut output, 2);
    for _ in 0..100 {
        render.process(&[], &mut output, 2);
    }
    let mut first = events.to_vec();
    first.push(0x2090_3064);
    render.process(&first, &mut output, 2);
    let mut sum = 0.0;
    for _ in 0..100 {
        render.process(&[], &mut output, 2);
        sum += output.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
    }
    (sum / (100.0 * 512.0)).sqrt()
}

/// Rewrites `key=value` lines of one `#cm=` section.
fn edit(state: &str, section: &str, changes: &[(&str, &str)]) -> String {
    let mut current = "";
    let mut out = String::new();
    for line in state.split_inclusive('\n') {
        let bare = line.trim_end_matches('\n');
        if let Some(name) = bare.strip_prefix("#cm=") {
            current = name;
        }
        let replaced = (current == section)
            .then(|| {
                let (key, _) = bare.split_once('=')?;
                let (_, value) = changes.iter().find(|(k, _)| *k == key)?;
                Some(format!("{key}={value}\n"))
            })
            .flatten();
        out.push_str(replaced.as_deref().unwrap_or(line));
        if line.starts_with("// Section for ugly") {
            current = "";
        }
    }
    out
}

/// Control A / B open a closed filter only through the CC saved as `[PCore] CtrlA` / `CtrlB`.
#[test]
#[ignore = "requires installed TyrellN6 CLAP"]
fn native_controls_follow_their_saved_ccs() {
    use crate::ffi;
    use std::ffi::{c_char, c_void, CStr};
    unsafe extern "C" fn wake(_: *mut c_void) {}
    unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
        let result = &mut *(user as *mut Option<Result<i32, String>>);
        *result = Some(if error.is_null() && id >= 0 {
            Ok(id)
        } else if error.is_null() {
            Err("creation failed".into())
        } else {
            Err(CStr::from_ptr(error).to_string_lossy().into_owned())
        });
    }
    let _dll = crate::native_library::load().unwrap();
    // Read only: the scanned bundle path of the installed TyrellN6 CLAP.
    let cache = crate::config::path()
        .unwrap()
        .with_file_name("plugin-list-cache.json");
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(cache).unwrap()).unwrap();
    let key: PluginKey = entries
        .into_iter()
        .filter_map(|entry| serde_json::from_value(entry["plugin"].clone()).ok())
        .find(|key: &PluginKey| supports(key))
        .unwrap();
    let host = unsafe { ffi::Host::new(wake, std::ptr::null_mut()) }.unwrap();
    let info = host.restore_plugin(&key).unwrap();
    let mut result: Option<Result<i32, String>> = None;
    unsafe {
        host.create_instance(
            info.index,
            48000,
            256,
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
    let initial = String::from_utf8(host.save_state(id).unwrap()).unwrap();
    let closed = edit(
        &initial,
        "Tyrell",
        &[("Cutoff", "0.00"), ("Res", "0.00"), ("KeyFol", "0.00")],
    );
    let route = |source: &str| {
        edit(
            &closed,
            "PCore",
            &[
                ("MMT1", "Tyrell:Cutoff"),
                ("MMS1", source),
                ("MMD1", "100.00"),
                ("MMS2", "0"),
                ("CtrlA", "20"),
                ("CtrlB", "21"),
            ],
        )
    };
    let quiet: Vec<u32> = [1, 2, 11, 20, 21].map(|c| cc(c, 0)).to_vec();
    let opened = |c: u8| [quiet.clone(), vec![cc(c, 127)]].concat();
    for (slot, source, moving, still) in [(0, "3", 20, 21), (1, "4", 21, 20)] {
        let changes = host.parameter_list_changes(id);
        host.load_state(id, route(source).as_bytes()).unwrap();
        // No rescan follows a patch change, unlike Surge XT: the app reads the state itself.
        assert_eq!(host.parameter_list_changes(id), changes);
        let saved = host.save_state(id).unwrap();
        let read = controls(&key, &saved);
        let mut expected = [None; MACROS];
        expected[slot] = Some(moving);
        assert_eq!(read.ccs(), expected);
        let processor = host.create_chain(id, &[], false, 48000, 256).unwrap();
        let mut render = unsafe { processor.audio_ref() };
        let closed = level(&mut render, &quiet);
        let open = level(&mut render, &opened(moving));
        let other = level(&mut render, &opened(still));
        eprintln!("slot {slot}: closed {closed:.5}, CC{moving} {open:.5}, CC{still} {other:.5}");
        assert!(open > closed * 1.25, "CC{moving} did not open the filter");
        assert!(
            (other / closed - 1.0).abs() < 0.1,
            "CC{still} moved the filter"
        );
        drop(processor);
        // The control positions stay out of the settings text, which ADR 0011
        // compares for favorites; only the compressed section changes.
        let played = host.save_state(id).unwrap();
        let settings = |state: &[u8]| {
            let text = String::from_utf8_lossy(state);
            text[..text.find("// Section for ugly").unwrap()].to_owned()
        };
        assert_eq!(settings(&saved), settings(&played));
    }
    host.destroy_instance(id);
}
