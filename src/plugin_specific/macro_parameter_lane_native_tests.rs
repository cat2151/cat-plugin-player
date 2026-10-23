//! Macro rows bound to an instrument's macro parameters move those parameters and the sound,
//! through the same sequencer lanes and batch the audio callback uses.
//! Run each case alone (`--exact`): a second native host in one process never finishes opening.
use super::macro_value_native_tests::{open, routing_favorite, wait_one_block, BLOCK};
use crate::{
    audio,
    ffi::{self, parameters::ParameterValue},
    instrument_macros::Instrument,
    macro_parameters::{self, Bindings},
    seq::{EventBuf, Sequencer},
    sequence_modulation::{MacroTarget, SequenceModulation, MACROS},
    SequencePattern, SequenceVelocity,
};

/// Level, and the RMS of the sample-to-sample change relative to it (higher is brighter).
#[derive(Clone, Copy, Debug)]
struct Sound {
    rms: f64,
    brightness: f64,
}

impl Sound {
    /// The larger relative difference of the two measures.
    fn distance(self, other: Self) -> f64 {
        let relative = |a: f64, b: f64| (a - b).abs() / a.max(b).max(f64::MIN_POSITIVE);
        relative(self.rms, other.rms).max(relative(self.brightness, other.brightness))
    }
}

struct Player<'a> {
    host: &'a ffi::Host,
    audio: ffi::ProcessorRef,
    bindings: Bindings,
    sequencer: Sequencer,
    ump: EventBuf,
    scratch: [ParameterValue; MACROS],
}

impl Player<'_> {
    /// Plays `blocks` blocks in real time with `numbers` on `choice`; returns the second
    /// half's sound and the levels sent before the last block. Choosing the same sweep again
    /// continues it; `restart` plays the phrase from its first note.
    fn play(
        &mut self,
        numbers: &[usize],
        choice: Option<SequenceModulation>,
        blocks: usize,
        restart: bool,
    ) -> (Sound, [Option<u8>; MACROS]) {
        if restart {
            self.sequencer.restart();
        }
        let mut lanes = [None; MACROS];
        for number in numbers {
            lanes[number - 1] = choice.map(|choice| (MacroTarget::Parameter, choice));
        }
        self.sequencer.set_macro_modulation(&lanes);
        let mut output = [0.0; BLOCK * 2];
        let (mut sum, mut slope, mut count) = (0.0, 0.0, 0);
        let mut previous = [0.0; 2];
        let mut last = [None; MACROS];
        for block in 0..blocks {
            self.ump.clear();
            self.sequencer.render(
                SequencePattern::Steps,
                SequenceVelocity::V100,
                BLOCK,
                &mut self.ump,
            );
            let levels = self.sequencer.macro_parameters();
            macro_parameters::send(&mut self.audio, &self.bindings, levels, &mut self.scratch)
                .unwrap();
            assert!(self.audio.process(self.ump.as_slice(), &mut output, 2));
            self.host.pump();
            wait_one_block();
            if block >= blocks / 2 {
                for frame in output.chunks(2) {
                    for (channel, sample) in frame.iter().enumerate() {
                        let sample = f64::from(*sample);
                        sum += sample * sample;
                        slope += (sample - previous[channel]).powi(2);
                        previous[channel] = sample;
                    }
                }
                count += output.len();
            }
            last = levels;
        }
        let sound = Sound {
            rms: (sum / count as f64).sqrt(),
            brightness: (slope / sum.max(f64::MIN_POSITIVE)).sqrt(),
        };
        (sound, last)
    }
}

/// The routed macros the player shows: without Floe's CC1 one.
fn rows(instrument: Instrument, state: &[u8]) -> Vec<usize> {
    instrument
        .routed("CLAP", state)
        .unwrap()
        .into_iter()
        .filter(|m| !m.ccs.contains(&1))
        .map(|m| m.number)
        .collect()
}

/// Reads each macro's (minimum, maximum, current) from the host.
fn read(
    host: &ffi::Host,
    id: i32,
    instrument: Instrument,
    numbers: &[usize],
) -> Vec<(f64, f64, f64)> {
    let parameters = host.parameters(id).unwrap();
    numbers
        .iter()
        .map(|&n| {
            let p = parameters
                .iter()
                .find(|p| p.eligible && instrument.parameter(n, &p.name))
                .unwrap();
            (p.minimum, p.maximum, p.current)
        })
        .collect()
}

fn check(id_of_plugin: &str) {
    let _dll = crate::native_library::load().unwrap();
    let instrument = Instrument::of("CLAP", id_of_plugin).unwrap();
    let (name, plugin, state) = routing_favorite(id_of_plugin);
    let (host, id) = open(&plugin, 48000);
    host.load_state(id, &state).unwrap();
    let numbers = rows(instrument, &state);
    assert!(!numbers.is_empty(), "{name}");
    let processor = host
        .create_chain(id, &[], false, 48000, BLOCK as u32)
        .unwrap();
    let bindings = macro_parameters::bind(&host, &processor, id, &plugin).unwrap();
    for n in &numbers {
        assert!(bindings[n - 1].is_some(), "{name}: macro {n} unbound");
    }
    let mut player = Player {
        host: &host,
        audio: unsafe { processor.audio_ref() },
        bindings,
        sequencer: Sequencer::new(48000),
        ump: EventBuf::new(),
        scratch: [ParameterValue::default(); MACROS],
    };
    player.play(&numbers, None, 50, false);

    // Level 0 twice tells how much the same notes vary by themselves.
    let mut sounds = Vec::new();
    for level in [0, 127, 0] {
        let choice = Some(SequenceModulation::Fixed(level));
        let (sound, _) = player.play(&numbers, choice, 100, true);
        for (minimum, maximum, current) in read(&host, id, instrument, &numbers) {
            let expected = if level == 0 { minimum } else { maximum };
            assert!(
                (current - expected).abs() < 1e-4,
                "{name}: {current} != {expected}"
            );
        }
        sounds.push(sound);
    }
    let (moved, unmoved) = (sounds[0].distance(sounds[1]), sounds[0].distance(sounds[2]));
    println!("{name} macros {numbers:?}: {sounds:?}; moved {moved:.4}, unmoved {unmoved:.4}");
    assert!(
        moved > 0.01 && moved > 3.0 * unmoved,
        "{name}: the macros must change the sound"
    );

    // The parameters follow the sweep: read back after every 25 blocks.
    let mut levels_seen = Vec::new();
    for _ in 0..6 {
        let (_, levels) = player.play(&numbers, Some(SequenceModulation::Sweep), 25, false);
        for (n, (minimum, maximum, current)) in
            numbers.iter().zip(read(&host, id, instrument, &numbers))
        {
            let level = levels[n - 1].unwrap();
            let expected = minimum + (maximum - minimum) * f64::from(level) / 127.0;
            assert!(
                (current - expected).abs() < 1e-4,
                "{name}: macro {n} at {level}: {current} != {expected}"
            );
            levels_seen.push(level);
        }
    }
    levels_seen.sort_unstable();
    levels_seen.dedup();
    println!("{name} sweep levels read back: {levels_seen:?}");
    assert!(levels_seen.len() >= 6, "{name}: the sweep must move");
    drop(player);
    drop(processor);
    host.destroy_instance(id);
}

/// The audio callback on the default output device moves the parameters, read back on this
/// thread while it plays.
fn live(id_of_plugin: &str) {
    let _dll = crate::native_library::load().unwrap();
    let instrument = Instrument::of("CLAP", id_of_plugin).unwrap();
    let (name, plugin, state) = routing_favorite(id_of_plugin);
    let output = audio::Output::open_default().unwrap();
    let (host, id) = open(&plugin, output.sample_rate());
    host.load_state(id, &state).unwrap();
    let numbers = rows(instrument, &state);
    let processor = host
        .create_chain(
            id,
            &[],
            false,
            output.sample_rate(),
            audio::MAX_BLOCK_FRAMES,
        )
        .unwrap();
    let bindings = macro_parameters::bind(&host, &processor, id, &plugin).unwrap();
    let lanes = |choice| {
        let mut lanes = [None; MACROS];
        for n in &numbers {
            lanes[n - 1] = Some((MacroTarget::Parameter, choice));
        }
        lanes
    };
    let voice = audio::Voice::start(
        &output,
        processor,
        SequencePattern::Steps,
        SequenceVelocity::V100,
        None,
        &lanes(SequenceModulation::Fixed(127)),
        bindings,
        std::time::Duration::ZERO,
        None,
        false,
        None,
        None,
    )
    .unwrap();
    let wait = |seconds: f64| {
        let until = std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds);
        while std::time::Instant::now() < until {
            host.pump();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    let at = |level: f64| {
        read(&host, id, instrument, &numbers)
            .into_iter()
            .map(|(minimum, maximum, current)| {
                (current - (minimum + (maximum - minimum) * level)).abs() < 1e-4
            })
            .collect::<Vec<_>>()
    };
    wait(2.0);
    assert!(voice.has_rendered(), "{name}: no audio callback");
    assert_eq!(at(1.0), vec![true; numbers.len()], "{name} at 127");
    voice.set_macro_modulation(&lanes(SequenceModulation::Fixed(0)));
    wait(0.5);
    assert_eq!(at(0.0), vec![true; numbers.len()], "{name} at 0");
    voice.set_macro_modulation(&lanes(SequenceModulation::Sweep));
    let mut seen = Vec::new();
    for _ in 0..8 {
        wait(0.3);
        seen.extend(
            read(&host, id, instrument, &numbers)
                .into_iter()
                .map(|p| p.2.to_bits()),
        );
    }
    seen.sort_unstable();
    seen.dedup();
    println!(
        "{name} macros {numbers:?}: {} values read during the sweep",
        seen.len()
    );
    assert!(
        seen.len() >= 6,
        "{name}: the sweep must move the parameters"
    );
    assert_eq!(voice.take_parameter_failure(), None);
    drop(voice);
    host.destroy_instance(id);
}

#[test]
#[ignore = "requires an audio output device, installed Vaporizer2 CLAP and saved favorites"]
fn native_vaporizer2_live_macro_rows_move_their_parameters() {
    live("com.vastdynamics.VAST2");
}

#[test]
#[ignore = "requires an audio output device, installed Six Sines CLAP and saved favorites"]
fn native_six_sines_live_macro_rows_move_their_parameters() {
    live("org.baconpaul.six-sines");
}

#[test]
#[ignore = "requires an audio output device, installed Floe CLAP and saved favorites"]
fn native_floe_live_macro_rows_move_their_parameters() {
    live("com.floe-audio.floe");
}

#[test]
#[ignore = "requires installed Vaporizer2 CLAP and saved favorites (read only)"]
fn native_vaporizer2_macro_rows_move_their_parameters() {
    check("com.vastdynamics.VAST2");
}

#[test]
#[ignore = "requires installed Six Sines CLAP and saved favorites (read only)"]
fn native_six_sines_macro_rows_move_their_parameters() {
    check("org.baconpaul.six-sines");
}

#[test]
#[ignore = "requires installed Floe CLAP and saved favorites (read only)"]
fn native_floe_macro_rows_move_their_parameters() {
    check("com.floe-audio.floe");
}
