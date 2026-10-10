use crate::{random_effect_catalog::Catalog, App};
use std::time::{Duration, Instant};

fn settle(app: &mut App, ctx: &eframe::egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.scanning
        || app.pending.is_some()
        || app.random_effect.busy()
        || app.effect_browser.desired.is_some()
    {
        assert!(Instant::now() < deadline, "{}", app.status);
        app.host.pump();
        app.handle_events();
        app.poll_random_effect();
        app.poll_effect_browser(ctx);
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn select(app: &mut App, display: &str) {
    let index = app
        .effect_browser
        .candidates
        .iter()
        .position(|candidate| candidate.preset.display.starts_with(display))
        .expect("required effect catalog preset missing");
    app.effect_browser.select(index);
}

fn effect_plugin(app: &App) -> Vec<String> {
    app.instances
        .iter()
        .filter(|instance| instance.kind == crate::plugin_list::PluginKind::Effect)
        .map(|instance| instance.plugin.id.clone())
        .collect()
}

#[test]
#[ignore = "requires installed Dexed and Dragonfly Hall/Room/Plate CLAP; real shim test"]
fn native_browse_effect_adds_then_replaces_its_slot_latest_only() {
    let _library = crate::native_library::load().unwrap();
    let directory = std::env::temp_dir().join(format!("cat-browse-effect-{}", std::process::id()));
    assert!(!directory.exists());
    let ctx = eframe::egui::Context::default();
    let mut app = App::prepare();
    app.config_path = Ok(directory.join("config.toml"));
    app.restore = None;
    app.restore_effect.clear();
    app.output = None;
    app.deferred_scan = false;
    app.start_scan(false);
    settle(&mut app, &ctx);
    app.random_effect_catalog = Catalog::from_candidates({
        let catalog = cmrt_core::AudioEffectCatalog::discover();
        catalog
            .presets()
            .iter()
            .filter(|preset| preset.display.starts_with("Dragonfly"))
            .filter_map(|preset| crate::random_effect_catalog::Candidate::new(&catalog, preset))
            .collect()
    });
    let source = app
        .plugins
        .iter()
        .position(|plugin| plugin.name == "Dexed" && plugin.format == "CLAP")
        .unwrap();
    app.load_plugin(source);
    settle(&mut app, &ctx);
    app.sequence_pattern = crate::SequencePattern::Off;

    app.open_effect_browser();
    app.poll_effect_browser(&ctx);
    assert_eq!(app.effect_browser.target, None, "no effect yet: add");
    assert!(!app.effect_browser.candidates.is_empty());
    select(&mut app, "Dragonfly Hall");
    settle(&mut app, &ctx);
    let ids = app.effect_ids();
    assert_eq!(ids.len(), 1, "{}", app.status);
    assert!(app.status.starts_with("Browse effect: "), "{}", app.status);
    assert_eq!(app.effect_browser.target, Some(ids[0]));
    let hall = effect_plugin(&app)[0].clone();

    // The next selection replaces the slot it added instead of adding another.
    select(&mut app, "Dragonfly Room");
    settle(&mut app, &ctx);
    assert_eq!(app.effect_ids().len(), 1, "{}", app.status);
    assert_ne!(effect_plugin(&app)[0], hall);
    assert_eq!(app.effect_browser.target, Some(app.effect_ids()[0]));

    // Selections made while one is preparing collapse to the latest.
    select(&mut app, "Dragonfly Plate");
    app.poll_effect_browser(&ctx);
    assert!(app.random_effect.busy());
    select(&mut app, "Dragonfly Hall");
    settle(&mut app, &ctx);
    assert_eq!(effect_plugin(&app), [hall], "{}", app.status);

    // Adding a plugin another slot already holds is refused without touching routing.
    app.effect_browser.target = None;
    let before = app.effect_ids();
    select(&mut app, "Dragonfly Hall");
    settle(&mut app, &ctx);
    assert_eq!(app.effect_ids(), before);
    assert!(
        app.status.contains("already connected in another slot"),
        "{}",
        app.status
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&directory);
}
