//! Read-only native probe: does changing `SURGE_DATA_HOME` between two
//! creations in one process change how long Surge XT takes to create?
use crate::{audio, config, ffi};
use std::{
    ffi::{c_char, c_void, CStr, OsString},
    path::PathBuf,
    time::{Duration, Instant},
};

const ENV: &str = "SURGE_DATA_HOME";

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let result = &mut *(user as *mut Option<Result<i32, String>>);
    *result = Some(if !error.is_null() {
        Err(CStr::from_ptr(error).to_string_lossy().into_owned())
    } else if id < 0 {
        Err("creation failed".into())
    } else {
        Ok(id)
    });
}

#[repr(C)]
struct ThreadEntry32 {
    size: u32,
    usage: u32,
    thread_id: u32,
    owner_process_id: u32,
    base_priority: i32,
    delta_priority: i32,
    flags: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> *mut c_void;
    fn Thread32First(snapshot: *mut c_void, entry: *mut ThreadEntry32) -> i32;
    fn Thread32Next(snapshot: *mut c_void, entry: *mut ThreadEntry32) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn GetCurrentProcessId() -> u32;
}

fn process_threads() -> usize {
    const TH32CS_SNAPTHREAD: u32 = 4;
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        assert!(!snapshot.is_null() && snapshot as isize != -1);
        let me = GetCurrentProcessId();
        let mut entry = ThreadEntry32 {
            size: std::mem::size_of::<ThreadEntry32>() as u32,
            usage: 0,
            thread_id: 0,
            owner_process_id: 0,
            base_priority: 0,
            delta_priority: 0,
            flags: 0,
        };
        let mut count = 0;
        let mut more = Thread32First(snapshot, &mut entry) != 0;
        while more {
            count += usize::from(entry.owner_process_id == me);
            more = Thread32Next(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        count
    }
}

/// The tree the player would use; `CAT_SURGE_MINIMAL_DATA_HOME` overrides.
fn minimal_data_home() -> PathBuf {
    if let Some(path) = std::env::var_os("CAT_SURGE_MINIMAL_DATA_HOME") {
        return PathBuf::from(path);
    }
    let cache = dirs::cache_dir()
        .unwrap()
        .join("cat-plugin-player")
        .join("cache");
    let before = std::env::var_os(ENV);
    let minimal = super::super::data_home::minimal_path(&cache)
        .unwrap()
        .expect("Surge XT data home must be minimizable");
    assert_eq!(
        std::env::var_os(ENV),
        before,
        "minimal_path must not set {ENV}"
    );
    assert!(minimal.starts_with(cache.join("surge-data-min-v2")) || before.is_some());
    minimal
}

fn set_data_home(value: Option<&OsString>) {
    match value {
        Some(value) => std::env::set_var(ENV, value),
        None => std::env::remove_var(ENV),
    }
}

#[test]
#[ignore = "requires installed Surge XT as the last played instrument; reads user data only"]
fn native_surge_data_home_switch_between_creations() {
    let _dll = crate::native_library::load().unwrap();
    let path = config::path().unwrap();
    let plugin = crate::status::Status::read(&path)
        .unwrap()
        .and_then(|status| status.last_played)
        .filter(super::matches_plugin)
        .expect("the last played instrument must be Surge XT");
    let minimal = minimal_data_home().into_os_string();
    let original = std::env::var_os(ENV);
    // Same arguments as App::restore_before_gui -> load_plugin.
    let rate =
        audio::Output::open_default().map_or(crate::FALLBACK_SAMPLE_RATE, |o| o.sample_rate());
    let host = unsafe { ffi::Host::new(wake, std::ptr::null_mut()) }.unwrap();
    let info = host.restore_plugin(&plugin).unwrap();
    eprintln!(
        "plugin={} [{}] bundle={} rate={rate} block={} original={original:?} minimal={minimal:?}",
        plugin.id,
        plugin.format,
        plugin.bundle_path,
        audio::MAX_BLOCK_FRAMES
    );
    let mut times = Vec::new();
    for (step, value) in [
        ("1 minimal", Some(&minimal)),
        ("2 original", original.as_ref()),
        ("3 minimal", Some(&minimal)),
        ("4 original", original.as_ref()),
    ] {
        set_data_home(value);
        let threads = process_threads();
        let mut result: Option<Result<i32, String>> = None;
        let start = Instant::now();
        unsafe {
            host.create_instance(
                info.index,
                rate,
                audio::MAX_BLOCK_FRAMES,
                created,
                &mut result as *mut _ as *mut c_void,
            )
        };
        let deadline = start + Duration::from_secs(30);
        while result.is_none() {
            assert!(Instant::now() < deadline, "creation timed out");
            host.pump_startup();
            std::thread::sleep(Duration::from_millis(1));
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let id = result.unwrap().unwrap();
        host.destroy_instance(id);
        eprintln!("step {step}: {ms:.1} ms threads_before_create={threads} env={value:?}");
        times.push(ms);
    }
    set_data_home(original.as_ref());
    assert_eq!(std::env::var_os(ENV), original);
    eprintln!("creation_ms={times:.1?}");
}
