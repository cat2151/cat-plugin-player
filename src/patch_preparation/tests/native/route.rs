//! Success keeps the established route while persistence uses owned state.
use super::*;
use crate::session::route_trace;

#[test]
#[ignore = "requires compiled CLAP fixture and S06_NATIVE_WORK_DIR; no device"]
fn native_success_keeps_one_route_including_save_failure() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let mut app = app(&root.join("single-route/config.toml"));
    let normal = key("normal");
    let other = key("context");
    let index = add(&mut app, &normal);
    add(&mut app, &other);
    app.load_plugin(index);
    wait(&mut app);
    for (target, value) in [(&normal, 5), (&other, 6), (&other, 7)] {
        let before = app.instrument_id().unwrap();
        route_trace::take();
        let started = Instant::now();
        app.apply_patch(prepared(target, &[value]));
        wait(&mut app);
        assert!(app.status.contains("Browse patch:"), "{}", app.status);
        assert_eq!(route_trace::take(), ["pause", "prepare"]);
        let after = app.instrument_id().unwrap();
        assert_eq!(before == after, target.id == normal.id || value == 7);
        assert_eq!(
            state_store::load(app.config_path.as_ref().unwrap(), target).unwrap(),
            Some(vec![value])
        );
        eprintln!("single route {}: {:?}", target.id, started.elapsed());
    }
    // A directory as the config file forces session persistence to fail, after
    // native success. It cannot force another pause or discard the applied state.
    let invalid = root.join("single-route/invalid-config");
    std::fs::create_dir_all(&invalid).unwrap();
    app.config_path = Ok(invalid);
    route_trace::take();
    app.apply_patch(prepared(&other, &[8]));
    wait(&mut app);
    assert!(
        app.status.contains("applied, saving failed"),
        "{}",
        app.status
    );
    assert_eq!(route_trace::take(), ["pause", "prepare"]);
    assert_eq!(
        app.host.save_state(app.instrument_id().unwrap()).unwrap(),
        [8]
    );
}
