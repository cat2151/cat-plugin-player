use super::*;

#[test]
fn patch_names_with_warning_words_do_not_create_a_success_footer() {
    let ctx = egui::Context::default();
    let mut browser = Browser {
        open: true,
        snapshot: Some(
            Snapshot::build(vec![candidate("a", "Alpha", "Error Bass", None)], &[]).unwrap(),
        ),
        ..Default::default()
    };
    browser.filter(false);
    browser.select(0);
    browser.requests.applied = Some(
        browser
            .requests
            .selected
            .as_ref()
            .unwrap()
            .candidate
            .clone(),
    );
    let (texts, _) = draw(&mut browser, &ctx, "Browse patch: Alpha — Error Bass");
    assert!(!texts.iter().any(|text| text.starts_with("Browse patch:")));
    let (texts, _) = draw(
        &mut browser,
        &ctx,
        "Preparing browse patch: Alpha — Error Bass...",
    );
    assert!(!texts
        .iter()
        .any(|text| text.starts_with("Preparing browse patch:")));
    let (texts, _) = draw(
        &mut browser,
        &ctx,
        "Browse patch: Alpha — Error Bass; applied, saving failed: disk full",
    );
    assert!(texts.iter().any(|text| text.contains("saving failed:")));
}

fn draw(browser: &mut Browser, ctx: &egui::Context, status: &str) -> (Vec<String>, f32) {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        },
        |ctx| browser.show(ctx, None, status),
    );
    let texts = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    let height = ctx.data(|data| {
        data.get_temp::<[egui::Rect; 3]>(egui::Id::new("browser_panes"))
            .unwrap()[2]
            .height()
    });
    (texts, height)
}

#[test]
fn ordinary_success_reclaims_footer_and_pending_identity_pink_and_errors_remain() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    browser.select(0);
    browser.requests.applied = Some(
        browser
            .requests
            .selected
            .as_ref()
            .unwrap()
            .candidate
            .clone(),
    );
    browser.requests.desired = None;
    draw(&mut browser, &ctx, "Browse patch: Alpha — Bass");
    let (texts, normal_height) = draw(&mut browser, &ctx, "Browse patch: Alpha — Bass");
    assert!(!texts.iter().any(|text| text.contains("Selected:")
        || text.contains("Last applied:")
        || text.contains("Browse patch:")));
    browser.requests.applied.as_mut().unwrap().path = "X:/different/state".into();
    let (texts, pending_height) = draw(
        &mut browser,
        &ctx,
        "Preparing browse patch: Alpha — Bass...",
    );
    assert!(texts
        .iter()
        .any(|text| text.starts_with("Not yet applied:") && text.contains("playing:")));
    assert!(normal_height > pending_height + 10.0);
    browser.requests.desired = browser.requests.selected.clone();
    browser.requests.next().unwrap();
    let (texts, _) = draw(
        &mut browser,
        &ctx,
        "Preparing browse patch: Alpha — Bass...",
    );
    assert!(texts
        .iter()
        .any(|text| text.starts_with("Not yet applied:")));
    browser.select(1);
    let (texts, _) = draw(&mut browser, &ctx, "Browse patch: Alpha — Bass");
    assert!(texts
        .iter()
        .any(|text| text.starts_with("Not yet applied: Beta — Heavy")
            && text.contains("Alpha — Bass")));
    assert!(texts
        .iter()
        .any(|text| text == "Space: confirm sample load"));
    assert!(browser.requests.desired.is_none());
    let (texts, _) = draw(&mut browser, &ctx, "Browse patch failed: test state error");
    assert!(texts.iter().any(|text| text.contains("test state error")));
    browser.select(0);
    browser.requests.applied = Some(
        browser
            .requests
            .selected
            .as_ref()
            .unwrap()
            .candidate
            .clone(),
    );
    let (texts, _) = draw(
        &mut browser,
        &ctx,
        "Browse patch: Alpha — Bass; applied, saving failed: disk full",
    );
    assert!(texts
        .iter()
        .any(|text| text.contains("applied, saving failed: disk full")));
    assert!(!texts
        .iter()
        .any(|text| text.starts_with("Not yet applied:")));
    browser.store = Some(Store::load(&Err("test config unavailable".into())));
    let (texts, _) = draw(&mut browser, &ctx, "Browse patch: Alpha — Bass");
    assert!(texts.iter().any(|text| text.contains("Saved conditions:")));
}
