use super::*;

fn frame(state: &mut FirstInstrument, ctx: &egui::Context, key: Option<egui::Key>) {
    let mut events = Vec::new();
    if let Some(key) = key {
        for pressed in [true, false] {
            events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
    }
    let _ = ctx.run(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ctx| {
            state.show(ctx, false);
        },
    );
}

#[test]
fn escape_dismisses_offer_without_installing_and_keeps_running_jobs() {
    let ctx = egui::Context::default();
    let mut state = FirstInstrument {
        text: Some(&text::ENGLISH),
        ..Default::default()
    };
    state.on_scan(&[]);
    frame(&mut state, &ctx, None);
    frame(&mut state, &ctx, Some(egui::Key::Escape));
    assert!(!state.dialog_open());
    assert!(!state.running());
    let (tx, rx) = mpsc::channel();
    state.job = Some(rx);
    state.dialog = Dialog::Installing {
        step: Step::Preparing,
        since: Instant::now(),
    };
    frame(&mut state, &ctx, None);
    frame(&mut state, &ctx, Some(egui::Key::Escape));
    assert!(!state.dialog_open());
    assert!(state.running());
    tx.send(Report::Done(Ok(()))).unwrap();
    assert_eq!(state.poll(), Some(Ok(())));
    assert!(!state.running());
    assert!(state.installed);
}

fn plugin(kind: PluginKind) -> PluginInfo {
    PluginInfo {
        index: 0,
        name: "Plugin".into(),
        vendor: "Vendor".into(),
        format: "CLAP".into(),
        id: "id".into(),
        bundle_path: String::new(),
        kind,
    }
}

#[test]
fn offers_only_when_no_instrument_was_found() {
    let mut state = FirstInstrument::default();
    state.on_scan(&[]);
    assert_eq!(state.dialog, Dialog::Offer);
    state.on_scan(&[plugin(PluginKind::Effect), plugin(PluginKind::Unknown)]);
    assert_eq!(state.dialog, Dialog::Offer);
    state.on_scan(&[plugin(PluginKind::Effect), plugin(PluginKind::Instrument)]);
    assert_eq!(state.dialog, Dialog::Hidden);
}

#[test]
fn a_running_install_keeps_its_dialog_through_rescans() {
    let (_sender, receiver) = mpsc::channel();
    let installing = || Dialog::Installing {
        step: Step::Downloading,
        since: Instant::now(),
    };
    let mut state = FirstInstrument {
        dialog: installing(),
        job: Some(receiver),
        installed: false,
        text: None,
    };
    state.on_scan(&[plugin(PluginKind::Instrument)]);
    assert!(matches!(state.dialog, Dialog::Installing { .. }));
}

#[test]
fn the_scan_after_an_install_loads_surge_clap_once() {
    let mut surge = plugin(PluginKind::Instrument);
    surge.id = SURGE_ID.into();
    let mut surge_vst3 = surge.clone();
    surge_vst3.format = "VST3".into();
    let plugins = [plugin(PluginKind::Effect), surge_vst3, surge];
    let loading = || Dialog::Installing {
        step: Step::Loading,
        since: Instant::now(),
    };
    let mut state = FirstInstrument {
        dialog: loading(),
        installed: true,
        ..Default::default()
    };
    assert_eq!(state.on_scan(&plugins), Some(2));
    assert!(matches!(state.dialog, Dialog::Installing { .. }));
    assert_eq!(state.on_scan(&plugins), None);
    assert_eq!(state.dialog, Dialog::Hidden);

    let mut missing = FirstInstrument {
        dialog: loading(),
        installed: true,
        ..Default::default()
    };
    assert_eq!(missing.on_scan(&[plugin(PluginKind::Effect)]), None);
    assert_eq!(missing.dialog, Dialog::Offer);
}

#[test]
fn steps_follow_the_recorded_winget_output() {
    // Piped `winget download` output on a Japanese Windows, in order.
    let lines = [
        ("見つかりました Surge XT [SurgeSynth.SurgeXT] バージョン 1.3.4", Step::Preparing),
        ("このアプリケーションは所有者からライセンス供与されます。", Step::Preparing),
        (
            "ダウンロード中 https://github.com/surge-synthesizer/releases-xt/releases/download/1.3.4/surge-xt-win64-1.3.4-setup.exe",
            Step::Downloading,
        ),
        ("インストーラーハッシュが正常に検証されました", Step::RunningInstaller),
        ("インストーラがダウンロードされました: X:\\dl\\setup.exe", Step::RunningInstaller),
    ];
    let mut step = Step::Preparing;
    for (line, expected) in lines {
        step = next_step(step, line);
        assert_eq!(step, expected, "{line}");
    }
}

#[test]
fn command_installs_the_exact_package_without_prompts() {
    let line = command_line();
    assert!(line.starts_with("winget install --id SurgeSynth.SurgeXT -e "));
    assert!(line.contains("--silent --disable-interactivity"));
    assert!(!line.contains("  "), "args are split on single spaces");
    let args: Vec<_> = line.split(' ').skip(1).collect();
    let custom = args.iter().position(|arg| *arg == "--custom");
    if cfg!(feature = "recording-mode") {
        assert_eq!(
            args[custom.expect("keep recorder running") + 1],
            "/NOCLOSEAPPLICATIONS"
        );
    } else {
        assert_eq!(custom, None);
    }
}
