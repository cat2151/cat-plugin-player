use super::*;

#[test]
#[ignore = "installed Surge XT; reads actual session and verifies browser/save/restart on an isolated copy, no stream or UI"]
fn native_real_session_browser_permission_and_restart() {
    let name = std::thread::current().name().unwrap().to_owned();
    if std::env::var("CAT_SURGE_DELAY_CASE").ok().as_deref() != Some(&name) {
        let root = directory();
        run_case(&root, &name, false);
        std::fs::remove_dir_all(root).unwrap();
        return;
    }
    let isolated =
        Path::new(&std::env::var(crate::native_library::process::ROOT).unwrap()).to_path_buf();
    let config = isolated.join("config.toml");
    let restarting = std::env::var_os("CAT_SURGE_DELAY_RESTART").is_some();
    if !restarting {
        let actual = dirs::config_local_dir().unwrap().join("cat-plugin-player");
        for file in ["config.toml", "status.json"] {
            std::fs::copy(actual.join(file), isolated.join(file)).unwrap();
        }
        std::fs::create_dir_all(isolated.join("states")).unwrap();
        for p in std::fs::read_dir(actual.join("states")).unwrap() {
            let p = p.unwrap().path();
            if p.is_file() {
                std::fs::copy(&p, isolated.join("states").join(p.file_name().unwrap())).unwrap();
            }
        }
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../cat-music-patterns/surge-xt-start-delay.json");
        let table = std::fs::read(source).unwrap();
        Table::parse(&table).unwrap();
        crate::state_store::atomic_write(&local_path(&config), &table).unwrap();
    }
    let library = crate::native_library::load().unwrap();
    let mut app = crate::App::prepare();
    app.output = None;
    app.restore_before_gui();
    assert!(app.pending.is_none(), "restoration must finish without GUI");
    let id = app
        .instrument_id()
        .expect("actual session has a restored instrument");
    if restarting {
        let key = &app.instances.iter().find(|i| i.id == id).unwrap().plugin;
        assert_eq!(
            app.surge_start_delay
                .delay(id, key, Duration::from_millis(100)),
            Duration::ZERO
        );
        let p = app.surge_start_delay.snapshot(id).unwrap();
        let state = crate::state_store::load(&config, key).unwrap().unwrap();
        assert_eq!(p.state_sha256, hash(&state));
        println!("real-session restart restored wait 0 with matching state");
        return;
    }
    let original = crate::status::Status::read(&config).unwrap().unwrap();
    assert!(original.last_played.is_some());
    let candidate = crate::patch_catalog::load()
        .unwrap()
        .into_iter()
        .find(|c| c.plugin_id == ID && c.display == "patches_factory/Basses/Attacky.fxp")
        .unwrap();
    let state = crate::state_request::catalog(ID, &candidate.bundle_path, &candidate.path).unwrap();
    app.apply_patch(Prepared { candidate, state });
    assert!(app.surge_start_delay.snapshot(id).is_some());
    // No device stream exists; apply Surge's queued state with memory-only processing.
    let processor = app.chain_processor(id, &[]).unwrap();
    let mut output = vec![0.0; 2048];
    assert!(unsafe { processor.audio_ref() }.process(&[], &mut output, 2));
    drop(processor);
    app.save_on_shutdown();
    let saved = crate::status::Status::read(&config).unwrap().unwrap();
    let p = saved.surge_start_delay.unwrap();
    assert_eq!(p.wait_ms, 0);
    let key = saved.last_played.unwrap();
    assert_eq!(
        p.state_sha256,
        hash(&crate::state_store::load(&config, &key).unwrap().unwrap())
    );
    drop(app);
    drop(library);
    run_case(&isolated, &name, true);
    println!(
        "real session restored, browser selected wait 0, saved and restored by a fresh process"
    );
}

fn run_case(root: &Path, name: &str, restart: bool) {
    let label = if restart { "restart" } else { "initial" };
    let stdout = root.join(format!("{label}.stdout.log"));
    let stderr = root.join(format!("{label}.stderr.log"));
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", name, "--nocapture"])
        .env("CAT_SURGE_DELAY_CASE", name)
        .env(crate::native_library::process::ROOT, root)
        .stdout(std::fs::File::create(&stdout).unwrap())
        .stderr(std::fs::File::create(&stderr).unwrap());
    if restart {
        command.env("CAT_SURGE_DELAY_RESTART", "1");
    } else {
        command.env_remove("CAT_SURGE_DELAY_RESTART");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().unwrap();
    let start = std::time::Instant::now();
    let result = loop {
        if let Some(exit) = child.try_wait().unwrap() {
            break exit;
        }
        if start.elapsed() > Duration::from_secs(60) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("{label} timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output = std::fs::read_to_string(stdout).unwrap();
    assert!(
        result.success() && output.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "{label}: {result}\n{output}\n{}",
        std::fs::read_to_string(stderr).unwrap()
    );
}
