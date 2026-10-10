use super::*;

#[test]
fn state_queue_delay_tracks_rate_and_format() {
    let mut key = PluginKey {
        format: "CLAP".into(),
        id: "org.baconpaul.six-sines".into(),
        name: String::new(),
        vendor: String::new(),
        bundle_path: String::new(),
    };
    for rate in [44_100, 48_000, 96_000] {
        let frames = (delay(&key, rate).as_secs_f64() * f64::from(rate)).round();
        assert_eq!(frames, f64::from(crate::audio::MAX_BLOCK_FRAMES));
    }
    key.id = "com.u-he.TyrellN6".into();
    assert_eq!(
        (delay(&key, 48_000).as_secs_f64() * 48_000.0).round(),
        4096.0
    );
    key.format = "VST3".into();
    assert!(delay(&key, 48_000).is_zero());
}
