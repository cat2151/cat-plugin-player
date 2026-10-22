use super::*;
use clap::Parser;

#[test]
fn cli_requires_explicit_input_and_excludes_gui_modes() {
    let valid = [
        "app",
        "--render-wav",
        "X:/out.wav",
        "--plugin-format",
        "CLAP",
        "--plugin-id",
        "instrument",
        "--plugin-path",
        "X:/instrument.clap",
        "--state-file",
        "X:/state.bin",
        "--wait-ms",
        "10",
    ];
    assert!(crate::cli::Cli::try_parse_from(valid).is_ok());
    for flag in [
        "--restore-state",
        "--patch-roots",
        "--resident",
        "--select-smf",
    ] {
        let mut args = valid.to_vec();
        args.push(flag);
        if matches!(flag, "--resident" | "--select-smf") {
            args.push("X:/input");
        }
        assert!(crate::cli::Cli::try_parse_from(args).is_err());
    }
    assert!(crate::cli::Cli::try_parse_from(&valid[..valid.len() - 2]).is_err());
    for value in ["-1", "NaN", "1.5", "4294967296"] {
        let mut args = valid.to_vec();
        *args.last_mut().unwrap() = value;
        assert!(crate::cli::Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn exact_note_position_and_capture_at_unaligned_waits() {
    for wait in [0, 1, 10, 25, 50, 100, 150, 200] {
        let mut frame = 0;
        let mut onset = None;
        let capture = capture(wait, |events, out| {
            let mut offset = 0;
            for word in events {
                if word & 0xffff_0000 == 0x0020_0000 {
                    offset += (word & 0xffff) as usize;
                }
                if word & 0xfff0_0000 == 0x2090_0000 {
                    onset = Some(frame + offset);
                }
            }
            for (i, stereo) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                let sample = onset
                    .filter(|&on| frame + i >= on)
                    .map_or(0.0, |on| (frame + i - on + 1) as f32);
                stereo.copy_from_slice(&[sample, -sample]);
            }
            frame += BLOCK;
            Ok(())
        })
        .unwrap();
        assert_eq!(onset, Some(wait as usize * 48));
        assert_eq!(capture.pcm.len(), CAPTURE * 2);
        assert_eq!(&capture.pcm[..2], &[1.0, -1.0]);
        assert_eq!(&capture.pcm[capture.pcm.len() - 2..], &[9600.0, -9600.0]);
    }
    assert!(capture(0, |_, _| Err("processor failed".into())).is_err());
    assert!(capture(0, |_, out| {
        out[0] = f32::NAN;
        Ok(())
    })
    .is_err());
}
