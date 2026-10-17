//! Launch the real player in resident mode and watch its top-level windows.
#![cfg(windows)]

#[path = "../shared/patch_identity.rs"]
mod patch_identity;
#[path = "../shared/resident_ipc.rs"]
#[allow(dead_code)]
mod resident_ipc;

/// `resident_ipc` names plugins by the host's key; this test never sends one.
mod host {
    pub type PluginKey = serde_json::Value;
}
// This window-lifetime test does not exchange plugin references or states.
mod patch_reference {
    pub type PatchReference = serde_json::Value;
}

use resident_ipc::{Listener, Open, ToPlayer};
use std::ffi::c_void;
use std::process::Command;
use std::time::{Duration, Instant};

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(
        callback: unsafe extern "system" fn(*mut c_void, isize) -> i32,
        data: isize,
    ) -> i32;
    fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
    fn IsWindowVisible(window: *mut c_void) -> i32;
    fn GetWindowTextW(window: *mut c_void, text: *mut u16, len: i32) -> i32;
    fn PostMessageW(window: *mut c_void, message: u32, w: usize, l: isize) -> i32;
}

struct Search {
    process: u32,
    found: Vec<(usize, String)>,
}

unsafe extern "system" fn collect(window: *mut c_void, data: isize) -> i32 {
    let search = unsafe { &mut *(data as *mut Search) };
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(window, &mut process) };
    if process == search.process && unsafe { IsWindowVisible(window) } != 0 {
        let mut text = [0u16; 256];
        let len = unsafe { GetWindowTextW(window, text.as_mut_ptr(), 256) };
        let title = String::from_utf16_lossy(&text[..len as usize]);
        search.found.push((window as usize, title));
    }
    1
}

/// Visible top-level windows with a title; winit's untitled helper window is skipped.
fn titles(process: u32) -> Vec<(usize, String)> {
    let mut search = Search {
        process,
        found: Vec::new(),
    };
    unsafe { EnumWindows(collect, &mut search as *mut Search as isize) };
    search.found.retain(|(_, title)| !title.is_empty());
    search.found
}

fn wait_until(process: u32, done: impl Fn(&[String]) -> bool) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let shown: Vec<_> = titles(process).into_iter().map(|(_, t)| t).collect();
        if done(&shown) || Instant::now() > deadline {
            return shown;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn press_escape(process: u32, title: &str) {
    let (window, _) = titles(process)
        .into_iter()
        .find(|(_, t)| t == title)
        .expect("browser window");
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const VK_ESCAPE: usize = 0x1b;
    unsafe {
        PostMessageW(window as *mut c_void, WM_KEYDOWN, VK_ESCAPE, 0x0001_0001);
        PostMessageW(
            window as *mut c_void,
            WM_KEYUP,
            VK_ESCAPE,
            0xc001_0001u32 as i32 as isize,
        );
    }
}

#[test]
#[ignore = "opens the player's browser windows on the desktop"]
fn resident_player_shows_hides_and_ends_with_the_editor() {
    let name = format!("windows-test-{}", std::process::id());
    let listener = Listener::create(&name).unwrap();
    let mut player = Command::new(env!("CARGO_BIN_EXE_cat-plugin-player"))
        .args(["--resident", &name])
        .spawn()
        .unwrap();
    let process = player.id();
    let (connected, link) = std::sync::mpsc::channel();
    std::thread::spawn(move || connected.send(listener.accept()));
    let started = Instant::now();
    let (mut editor, editor_in) = loop {
        if let Ok(link) = link.try_recv() {
            break link.unwrap();
        }
        if let Some(status) = player.try_wait().unwrap() {
            panic!("player exited before connecting: {status}");
        }
        assert!(started.elapsed() < Duration::from_secs(30), "no connection");
        std::thread::sleep(Duration::from_millis(5));
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(
            titles(process),
            Vec::new(),
            "nothing is shown before a request"
        );
        for _ in 0..2 {
            editor
                .send(&ToPlayer::Open(Open::Patch { track: 0 }))
                .unwrap();
            let shown = wait_until(process, |t| t == ["Browse patches"]);
            assert_eq!(shown, ["Browse patches"]);
            press_escape(process, "Browse patches");
            assert_eq!(
                wait_until(process, <[String]>::is_empty),
                Vec::<String>::new()
            );
            assert!(player.try_wait().unwrap().is_none(), "Esc must not end it");
        }
        editor
            .send(&ToPlayer::Open(Open::MasterEffect { slot: None }))
            .unwrap();
        let shown = wait_until(process, |t| t == ["Browse master effects"]);
        assert_eq!(shown, ["Browse master effects"]);
    }));
    drop((editor, editor_in));
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = player.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() > deadline {
            let _ = player.kill();
            let _ = player.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
    assert!(
        status.is_some_and(|s| s.success()),
        "player must exit once the editor closes: {status:?}"
    );
}
