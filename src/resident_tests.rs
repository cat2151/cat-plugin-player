//! Drive a resident player through its real pipes and the rendered browsers.
use super::*;
use crate::resident_ipc::{Listener, Receiver, Sender, ToEditor};
use eframe::egui;
use std::time::{Duration, Instant};

fn link(test: &str) -> ((Sender, Receiver), Resident) {
    let name = format!("resident-test-{}-{test}", std::process::id());
    let listener = Listener::create(&name).unwrap();
    let player = std::thread::spawn(move || Resident::connect(&name).unwrap());
    let editor = listener.accept().unwrap();
    (editor, player.join().unwrap())
}

fn app(test: &str) -> (App, (Sender, Receiver)) {
    let root = crate::selection_tests::directory(test);
    let mut app = App::prepare_mode(true);
    app.config_path = Ok(root.join("config.toml"));
    app.deferred_scan = false;
    let (editor, resident) = link(test);
    app.resident = Some(resident);
    (app, editor)
}

/// Polls like the GUI does until the editor's request has arrived.
fn request(app: &mut App, editor: &mut Sender, open: Open) {
    editor.send(&ToPlayer::Open(open)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.resident.as_ref().unwrap().shown != Some(open) {
        assert!(!app.poll_resident(), "link closed early");
        assert!(Instant::now() < deadline, "request did not arrive");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>, close: bool) {
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1100.0, 750.0),
        )),
        events,
        ..Default::default()
    };
    if close {
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .events
            .push(egui::ViewportEvent::Close);
    }
    let _ = ctx.run(input, |ctx| {
        app.browser_ui(ctx);
        app.effect_browser_ui(ctx);
    });
}

fn escape() -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }]
}

fn shown(app: &App) -> Option<Open> {
    app.resident.as_ref().unwrap().shown
}

#[test]
fn show_hide_show_again_then_end_when_the_editor_closes() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, editor_in)) = app("resident states");
    let ctx = egui::Context::default();
    assert_eq!(shown(&app), None);
    assert!(!app.patch_browser.open && !app.effect_browser.open);

    let patch = Open::Patch { track: 1 };
    request(&mut app, &mut editor, patch);
    assert!(app.patch_browser.open && !app.effect_browser.open);
    assert!(app.take_browser_focus(crate::cli::Browser::Patch));
    frame(&mut app, &ctx, Vec::new(), false);
    assert_eq!(shown(&app), Some(patch), "a frame alone must not hide it");

    // Esc hides the browser; the player stays connected.
    frame(&mut app, &ctx, escape(), false);
    assert_eq!(shown(&app), None);
    assert!(!app.patch_browser.open && !app.effect_browser.open);
    assert!(!app.poll_resident());

    // A second request shows a browser again, master effects this time.
    let master = Open::MasterEffect { slot: Some(0) };
    request(&mut app, &mut editor, master);
    assert!(app.effect_browser.open && !app.patch_browser.open);
    assert!(app.selecting_master());
    frame(&mut app, &ctx, Vec::new(), false);
    assert!(
        !app.patch_browser.open,
        "master effects have no patch browser"
    );

    // The window's close button hides it too.
    frame(&mut app, &ctx, Vec::new(), true);
    assert_eq!(shown(&app), None);
    assert!(!app.effect_browser.open);
    assert!(!app.selecting_master());

    let effect = Open::Effect {
        track: 2,
        slot: None,
    };
    request(&mut app, &mut editor, effect);
    assert!(app.effect_browser.open && !app.selecting_master());
    // Switching straight to the patch browser closes the effect browser.
    request(&mut app, &mut editor, patch);
    assert!(app.patch_browser.open && !app.effect_browser.open);

    // The editor goes away: the player is told to end.
    drop((editor, editor_in));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !app.poll_resident() {
        assert!(Instant::now() < deadline, "disconnect was not noticed");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn the_patch_catalog_loads_without_audio() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let (mut app, _editor) = app("resident catalog");
    assert!(app.instances.is_empty());
    app.poll_patch_catalog(&egui::Context::default());
    assert_ne!(
        app.patch_catalog.notice(),
        Some("Catalog will load after the first audio callback.")
    );
}

fn plugin(id: &str, kind: crate::plugin_list::PluginKind) -> crate::ffi::PluginInfo {
    crate::ffi::PluginInfo {
        index: 0,
        name: id.into(),
        vendor: String::new(),
        format: "CLAP".into(),
        id: id.into(),
        bundle_path: format!("X:/plugins/{id}.clap"),
        kind,
    }
}

fn selected(editor_in: &mut Receiver) -> (resident_ipc::Selected, Vec<u8>) {
    let ToEditor::Selected(selected) = editor_in.receive().unwrap().unwrap() else {
        panic!("expected selection")
    };
    let state = resident_ipc::read_state(&selected.state).unwrap();
    (selected, state)
}

/// Runs the GUI's polling until a selection has gone to the editor.
fn settle(app: &mut App, ctx: &egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !app.resident_waiting() {
        app.poll_browser_patch(ctx);
        app.poll_effect_browser(ctx);
        app.poll_effect_preparation();
        assert!(
            Instant::now() < deadline,
            "nothing was sent: {}",
            app.status
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Polls like the GUI does until the editor's answer has arrived.
fn answer(app: &mut App, editor: &mut Sender, result: Result<(), String>) {
    editor.send(&ToPlayer::Applied(result)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.resident_waiting() {
        assert!(!app.poll_resident(), "link closed early");
        assert!(Instant::now() < deadline, "answer did not arrive");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn chosen_patches_and_effects_go_to_the_editor_instead_of_loading() {
    if crate::native_library::process::isolated() {
        return;
    }
    use crate::plugin_list::PluginKind;
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut editor_in)) = app("resident offers");
    app.plugins = vec![
        plugin("synth", PluginKind::Instrument),
        plugin("id.Surge", PluginKind::Effect),
    ];
    let ctx = egui::Context::default();

    // A patch prepared while the patch browser is shown.
    let patch = Open::Patch { track: 2 };
    request(&mut app, &mut editor, patch);
    let candidate =
        crate::patch_browser_catalog::tests::candidate("synth", "Synth", "Bass 1", None);
    let state: Vec<u8> = (0..1_300_001u32).map(|i| i as u8).collect();
    app.patch_browser.requests.select(candidate.clone(), 0);
    let ticket = app.patch_browser.requests.next().unwrap();
    let prepared = state.clone();
    app.remember_preparation_target();
    app.patch_browser
        .preparation
        .start_with(ticket.candidate, ctx.clone(), move |_| Ok(prepared));
    settle(&mut app, &ctx);
    assert!(app.resident_waiting());
    let (sent, received) = selected(&mut editor_in);
    assert_eq!(sent.target, patch);
    assert_eq!(sent.plugin.id, "synth");
    assert_eq!(sent.preset_name.as_deref(), Some("Bass 1"));
    assert_eq!(received, state);
    answer(&mut app, &mut editor, Ok(()));
    assert_eq!(
        app.patch_browser.requests.applied.as_ref().unwrap().display,
        "Bass 1"
    );
    assert_eq!(app.status, "Browse patch: Synth — Bass 1");
    assert!(app.instances.is_empty(), "the player never loads a plugin");

    // An effect for a master slot; the editor refuses it.
    let master = Open::MasterEffect { slot: Some(1) };
    request(&mut app, &mut editor, master);
    let mut effect =
        crate::effect_browser::tests::candidate("Surge", "Big Hall", "Space", "Reverb");
    effect.state = Some(vec![9; 10].into());
    app.effect_browser.desired = Some(effect);
    settle(&mut app, &ctx);
    let (sent, received) = selected(&mut editor_in);
    assert_eq!(sent.target, master);
    assert_eq!(sent.plugin.id, "id.Surge");
    assert_eq!(received, vec![9; 10]);
    answer(&mut app, &mut editor, Err("refused".into()));
    assert_eq!(app.status, "Browse effect failed: refused");
    assert!(app.instances.is_empty());

    // Enter confirms and closes the effect browser: the resident browser hides.
    let enter = vec![egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }];
    assert!(app.take_browser_focus(crate::cli::Browser::Effect));
    frame(&mut app, &ctx, Vec::new(), false);
    frame(&mut app, &ctx, enter, false);
    assert!(!app.effect_browser.open);
    assert_eq!(shown(&app), None);
}

/// One GUI frame as `App::update` runs it, minus what needs a real window.
fn tick(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
    assert!(!app.poll_resident(), "link closed early");
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1100.0, 750.0),
        )),
        events,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        app.host.pump();
        app.handle_events();
        app.initialize_favorites();
        app.poll_effect_preparation();
        if let Some(notice) = app
            .effect_catalog
            .update(!app.scanning && !app.deferred_scan && !app.restoring, ctx)
        {
            app.status = notice;
        }
        app.poll_patch_catalog(ctx);
        app.poll_catalog_startup(ctx);
        app.poll_browser_load();
        app.browser_ui(ctx);
        app.poll_browser_patch(ctx);
        app.poll_effect_browser(ctx);
        app.effect_browser_ui(ctx);
    });
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }
}

/// An editor that applies every selection after `delay`, as the real one does.
fn answering_editor(
    editor: std::sync::Arc<std::sync::Mutex<Sender>>,
    mut editor_in: Receiver,
    delay: Duration,
) -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let record = seen.clone();
    std::thread::spawn(move || {
        while let Ok(Some(ToEditor::Selected(selected))) = editor_in.receive::<ToEditor>() {
            resident_ipc::read_state(&selected.state).unwrap();
            record
                .lock()
                .unwrap()
                .push(selected.preset_name.unwrap_or_default());
            std::thread::sleep(delay);
            if editor
                .lock()
                .unwrap()
                .send(&ToPlayer::Applied(Ok(())))
                .is_err()
            {
                return;
            }
        }
    });
    seen
}

fn settled(app: &App) -> bool {
    let requests = &app.patch_browser.requests;
    requests.selected.as_ref().is_some_and(|selected| {
        requests.applied.as_ref().is_some_and(|applied| {
            crate::patch_browser_requests::same(&selected.candidate, applied)
        })
    })
}

#[test]
#[ignore = "needs the installed plugins and patch catalog"]
fn real_patch_choices_each_reach_the_editor() {
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, editor_in)) = app("resident real patches");
    // The installed plugins come from the user's cache; saves stay in the test directory.
    let scratch = std::mem::replace(
        &mut app.config_path,
        App::prepare_mode(true).config_path.clone(),
    );
    app.load_plugin_cache();
    app.config_path = scratch;
    assert!(!app.plugins.is_empty(), "no plugin cache");
    let ctx = egui::Context::default();
    request(&mut app, &mut editor, Open::Patch { track: 1 });
    let editor = std::sync::Arc::new(std::sync::Mutex::new(editor));
    let seen = answering_editor(editor.clone(), editor_in, Duration::from_millis(600));
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.patch_browser.results.is_empty() {
        assert!(
            Instant::now() < deadline,
            "no patches listed: {}",
            app.status
        );
        tick(&mut app, &ctx, Vec::new());
        std::thread::sleep(Duration::from_millis(5));
    }
    let presets = app.patch_browser.snapshot.as_ref().unwrap().presets[0].len();
    for round in 0..8u64 {
        match round % 3 {
            0 => tick(&mut app, &ctx, vec![key(egui::Key::R)]),
            // A Preset row clicked in the left pane.
            1 => {
                app.patch_browser.preset = 1 + (round as usize * 7) % (presets - 1);
                app.patch_browser.filter(true);
                tick(&mut app, &ctx, Vec::new());
            }
            // The editor's patch button pressed again.
            _ => {
                let open = Open::Patch { track: 1 };
                editor.lock().unwrap().send(&ToPlayer::Open(open)).unwrap();
                tick(&mut app, &ctx, Vec::new());
            }
        }
        let until = Instant::now() + Duration::from_millis(150 + round * 120);
        while Instant::now() < until {
            tick(&mut app, &ctx, Vec::new());
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    while !settled(&app) {
        assert!(
            Instant::now() < deadline,
            "last choice never applied; status={:?} sent={:?} selected={:?} preparing={} applying={} waiting={}",
            app.status,
            seen.lock().unwrap(),
            app.patch_browser.requests.selected.as_ref().map(|t| &t.candidate.display),
            app.patch_browser.requests.preparing.is_some(),
            app.patch_browser.requests.applying.is_some(),
            app.resident_waiting(),
        );
        tick(&mut app, &ctx, Vec::new());
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[path = "resident_escape_tests.rs"]
mod escape;

#[path = "resident_panes_pipe_tests.rs"]
mod panes;

#[path = "resident_cursor_pipe_tests.rs"]
mod cursor;
