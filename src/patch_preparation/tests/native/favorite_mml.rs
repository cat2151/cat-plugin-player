//! Saved MML transactions through real state APIs; deliberately no audio output.
use super::*;
use crate::favorites_store::FavoriteCapture;

#[test]
#[ignore = "requires CLAP fixture and isolated S06_NATIVE_WORK_DIR"]
fn native_favorite_mml_restore_and_rejection() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    for stopped in [true, false] {
        let path = root
            .join(format!("favorite-mml-{stopped}"))
            .join("config.toml");
        let mut app = app(&path);
        app.mml_input = Default::default();
        app.sequence_pattern = SequencePattern::Steps;
        let normal = key("normal");
        let other = key("context");
        let index = add(&mut app, &normal);
        add(&mut app, &other);
        app.load_plugin(index);
        wait(&mut app);
        let capture = |text: &str| FavoriteCapture {
            sequence_pattern: SequencePattern::Off,
            selected_sequence: Some(SequencePattern::Custom),
            mml: Some(text.into()),
            sweep_cc1: false,
        };
        let first = app
            .favorites
            .library
            .add(&path, normal.clone(), false, &[4], capture("o5 l8 c"))
            .unwrap();
        let second = app
            .favorites
            .library
            .add(&path, other.clone(), false, &[5], capture("C F G"))
            .unwrap();
        for favorite in [&first, &second] {
            app.sequence_pattern = if stopped {
                SequencePattern::Off
            } else {
                SequencePattern::Steps
            };
            app.load_favorite(&favorite.id);
            wait(&mut app);
            assert!(app.status.contains("Loaded favorite:"), "{}", app.status);
            assert_eq!(app.mml_input.confirmed, favorite.mml.as_deref().unwrap());
            assert_eq!(app.selected_sequence, SequencePattern::Custom);
            assert_eq!(
                app.sequence_pattern,
                if stopped {
                    SequencePattern::Off
                } else {
                    SequencePattern::Custom
                }
            );
            let expected =
                crate::timed_sequence::Phrase::parse(&app.mml_input.confirmed, 48000).unwrap();
            assert_eq!(
                app.mml_input.phrase.as_ref().unwrap().events,
                expected.events
            );
            let saved = crate::status::Status::load(&path).unwrap();
            assert_eq!(saved.mml, app.mml_input.confirmed);
            assert_eq!(saved.selected_sequence, SequencePattern::Custom);
        }
        let source = app.instrument_id().unwrap();
        let text = app.mml_input.confirmed.clone();
        let phrase = app.mml_input.phrase.clone().unwrap();
        let transport = app.sequence_pattern;
        let before = files(&root);
        // Invalid Custom has to fail before even autosaving the previous plugin.
        let mut invalid = first.clone();
        invalid.id = "bad-conversion".into();
        invalid.mml = Some(" ".into());
        app.favorites.library.entries.push(invalid);
        app.load_favorite("bad-conversion");
        assert_eq!(app.instrument_id(), Some(source));
        assert_eq!(files(&root), before);
        // Rejected native state: both same-plugin and replacement paths retain MML.
        for plugin in [&other, &normal] {
            let bad = app
                .favorites
                .library
                .add(&path, plugin.clone(), false, &[0], capture("d"))
                .unwrap();
            let before = files(&root);
            app.load_favorite(&bad.id);
            wait(&mut app);
            assert!(app.status.contains("CLAP state load"), "{}", app.status);
            assert_eq!(app.instrument_id(), Some(source));
            assert_eq!(app.mml_input.confirmed, text);
            assert!(std::sync::Arc::ptr_eq(
                app.mml_input.phrase.as_ref().unwrap(),
                &phrase
            ));
            assert_eq!(app.selected_sequence, SequencePattern::Custom);
            assert_eq!(app.sequence_pattern, transport);
            assert_eq!(app.host.save_state(source).unwrap(), [5]);
            // Replacement autosaves the prior plugin/history before creation;
            // failed favorite contents must never enter the saved session.
            let status = crate::status::Status::load(&path).unwrap();
            assert_eq!(status.mml, text);
            if plugin.id == other.id {
                assert_eq!(files(&root), before);
            }
        }
        // Duplicate chain IDs are rejected by the real shim before processor
        // lookup. Exercise voice preparation failure after saved MML is applied.
        // Without an audio output the same-instance path has no voice to
        // prepare. The replacement path still validates the chain.
        {
            let favorite = &first;
            app.instances.push(crate::Instance {
                id: source,
                label: "invalid duplicate".into(),
                plugin: other.clone(),
                kind: PluginKind::Effect,
                voice: None,
                sweep_cc1: false,
            });
            app.load_favorite(&favorite.id);
            wait(&mut app);
            assert!(app.status.contains("Cannot connect"), "{}", app.status);
            app.instances.retain(|i| i.kind != PluginKind::Effect);
            assert_eq!(app.instrument_id(), Some(source));
            assert_eq!(app.mml_input.confirmed, text);
            assert!(std::sync::Arc::ptr_eq(
                app.mml_input.phrase.as_ref().unwrap(),
                &phrase
            ));
            assert_eq!(app.selected_sequence, SequencePattern::Custom);
            assert_eq!(app.sequence_pattern, transport);
            assert_eq!(app.host.save_state(source).unwrap(), [5]);
            assert_eq!(crate::status::Status::load(&path).unwrap().mml, text);
        }
        let effect = add(&mut app, &key("effect"));
        app.plugins[effect].kind = PluginKind::Effect;
        app.load_plugin(effect);
        wait(&mut app);
        let fx = app.effect_id().unwrap();
        app.add_favorite(fx);
        let favorite = app.favorites.library.entries[0].id.clone();
        app.load_favorite(&favorite);
        assert_eq!(app.mml_input.confirmed, text);
        assert!(std::sync::Arc::ptr_eq(
            app.mml_input.phrase.as_ref().unwrap(),
            &phrase
        ));
    }
}
