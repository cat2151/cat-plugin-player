use super::*;

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

fn frame(ctx: &egui::Context, events: Vec<Event>, enabled: bool) -> (Vec<(Menu, Key)>, bool, bool) {
    let mut actions = Vec::new();
    let mut exit = false;
    let mut space = false;
    let mut events = events;
    let releases: Vec<_> = events
        .iter()
        .filter_map(|event| {
            if let Event::Key { key, modifiers, .. } = event {
                Some(Event::Key {
                    key: *key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: *modifiers,
                })
            } else {
                None
            }
        })
        .collect();
    events.extend(releases);
    let _ = ctx.run(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ctx| {
            keyboard(ctx, enabled);
            exit = crate::selection::exit_key(ctx);
            space = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Space));
            egui::CentralPanel::default().show(ctx, |ui| {
                for menu in [Menu::Settings, Menu::Analysis] {
                    menu.button(ui, |ui| {
                        ui.label("Menu");
                        let keys: &[Key] = match menu {
                            Menu::Settings => &[Key::O, Key::N, Key::D, Key::Y],
                            Menu::Analysis => &[Key::A, Key::P, Key::C, Key::Z, Key::G],
                        };
                        for &key in keys {
                            if menu.take(ctx, key) {
                                actions.push((menu, key));
                            }
                        }
                    });
                }
            });
        },
    );
    (actions, exit, space)
}

#[test]
fn prefixes_open_without_executing_and_actions_close_only_their_menu() {
    for (menu, prefix, keys) in [
        (Menu::Settings, Key::D, vec![Key::O, Key::N, Key::D, Key::Y]),
        (
            Menu::Analysis,
            Key::A,
            vec![Key::A, Key::P, Key::C, Key::Z, Key::G],
        ),
    ] {
        for action in keys {
            let ctx = egui::Context::default();
            assert!(frame(&ctx, vec![key(prefix)], true).0.is_empty());
            assert!(ctx.memory(|m| m.is_popup_open(menu.id())));
            let result = frame(&ctx, vec![key(action), Event::Text("x".into())], true);
            assert!(result.0 == vec![(menu, action)]);
            assert!(!ctx.memory(|m| m.any_popup_open()));
            assert!(ctx.input(|i| i.events.iter().all(|e| !matches!(e, Event::Text(_)))));
        }
    }
}

#[test]
fn old_direct_keys_do_nothing_and_wrong_menu_keys_cannot_escape() {
    let ctx = egui::Context::default();
    for k in [Key::O, Key::N, Key::Y, Key::P, Key::C, Key::Z, Key::G] {
        assert!(frame(&ctx, vec![key(k)], true).0.is_empty());
        assert!(!ctx.memory(|m| m.any_popup_open()));
    }
    frame(&ctx, vec![key(Key::D)], true);
    for k in [Key::A, Key::Space, Key::Q] {
        let result = frame(&ctx, vec![key(k)], true);
        assert!(result.0.is_empty() && !result.1 && !result.2);
        assert!(ctx.memory(|m| m.is_popup_open(Menu::Settings.id())));
    }
    assert!(!frame(&ctx, vec![key(Key::Escape)], true).1);
    assert!(!ctx.memory(|m| m.any_popup_open()));
    assert!(frame(&ctx, vec![key(Key::Escape)], true).1);
    assert!(frame(&ctx, vec![key(Key::Space)], true).2);
}

#[test]
fn disabled_text_modified_and_repeated_prefixes_do_not_open() {
    let ctx = egui::Context::default();
    for prefix in [Key::D, Key::A] {
        frame(&ctx, vec![key(prefix)], false);
        assert!(!ctx.memory(|m| m.any_popup_open()));
        let mut modified = key(prefix);
        if let Event::Key { modifiers, .. } = &mut modified {
            *modifiers = egui::Modifiers::CTRL;
        }
        frame(&ctx, vec![modified], true);
        assert!(!ctx.memory(|m| m.any_popup_open()));
        let mut repeat = key(prefix);
        if let Event::Key { repeat, .. } = &mut repeat {
            *repeat = true;
        }
        // egui derives repeat from held keys rather than trusting RawInput's flag.
        let _ = ctx.run(
            egui::RawInput {
                events: vec![key(prefix)],
                ..Default::default()
            },
            |_| {},
        );
        frame(&ctx, vec![repeat], true);
        assert!(!ctx.memory(|m| m.any_popup_open()));
    }
    let mut text = String::new();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        });
    });
    frame(&ctx, vec![key(Key::D), Event::Text("d".into())], true);
    assert!(!ctx.memory(|m| m.any_popup_open()));
    assert!(ctx.input(|i| i
        .events
        .iter()
        .any(|e| matches!(e, Event::Text(t) if t == "d"))));
}

#[test]
fn fast_two_strokes_in_one_frame_work_and_disabled_state_cancels() {
    let ctx = egui::Context::default();
    assert!(frame(&ctx, vec![key(Key::A), key(Key::C)], true).0 == vec![(Menu::Analysis, Key::C)]);
    frame(&ctx, vec![key(Key::D)], true);
    frame(&ctx, vec![], false);
    assert!(!ctx.memory(|m| m.any_popup_open()));
    assert!(frame(&ctx, vec![key(Key::O)], true).0.is_empty());
}

#[test]
fn mouse_open_uses_the_same_key_scope_and_outside_click_closes() {
    let ctx = egui::Context::default();
    let draw = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                keyboard(ctx, true);
                egui::CentralPanel::default().show(ctx, |ui| {
                    Menu::Settings.button(ui, |ui| {
                        ui.label("Settings contents");
                    });
                });
            },
        )
    };
    draw(vec![]);
    let output = draw(vec![]);
    let button = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                if text.galley.job.text == "☰" {
                    return Some(text.pos + text.galley.size() / 2.0);
                }
            }
            None
        })
        .unwrap();
    let click = |position| {
        for pressed in [true, false] {
            draw(vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
    };
    click(button);
    assert!(ctx.memory(|m| m.is_popup_open(Menu::Settings.id())));
    assert!(frame(&ctx, vec![key(Key::O)], true).0 == vec![(Menu::Settings, Key::O)]);
    draw(vec![key(Key::D)]);
    assert!(ctx.memory(|m| m.is_popup_open(Menu::Settings.id())));
    click(egui::pos2(850.0, 550.0));
    assert!(!ctx.memory(|m| m.any_popup_open()));
}

#[test]
fn settings_menu_has_readable_width_beside_a_right_aligned_button() {
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 600.0));
    let label = "Show only CLAP for duplicate plugins (D)";
    let draw = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ctx| {
                keyboard(ctx, true);
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Favorites / History / Plugins");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            Menu::Settings.button(ui, |ui| {
                                ui.label("Settings");
                                ui.checkbox(&mut false, "Reorder favorites (O)");
                                let _ = ui.button("Sort by plugin name (N)");
                                ui.checkbox(&mut true, label);
                                let _ = ui.button("Rescan plugins... (Y)");
                            });
                        });
                    });
                });
            },
        )
    };
    draw(vec![key(Key::D)]);
    draw(vec![]);
    let output = draw(vec![]);
    let text = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
            _ => None,
        })
        .expect("settings item must be rendered");
    assert!(
        text.galley.size().x > 200.0,
        "menu text was squeezed: {:?}",
        text.galley.size()
    );
    assert!(
        text.galley.rows.len() <= 2,
        "menu text wrapped into a tall column"
    );
    assert!(screen.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())));
}
