//! Parse commands before initializing audio, plugins, or the GUI.

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about = "オーディオプラグインを手軽に演奏するアプリ")]
pub struct Cli {
    /// Read a patch reference from stdin and return opaque state on stdout.
    #[arg(long, conflicts_with_all = ["resident", "select_smf"])]
    pub restore_state: bool,
    /// Print where installed plugins keep their patches, as JSON on stdout.
    #[arg(long, conflicts_with_all = ["restore_state", "resident", "select_smf"])]
    pub patch_roots: bool,
    /// Stay hidden and show browsers when the editor listening on this name asks.
    #[arg(long, conflicts_with = "select_smf")]
    pub resident: Option<String>,
    #[arg(long, requires = "preserve_position")]
    pub select_smf: Option<std::path::PathBuf>,
    /// Edit file-wide effects while preserving the preview track chain.
    #[arg(long, requires = "select_smf")]
    pub master_effects: bool,
    #[arg(long, value_enum, requires = "select_smf")]
    pub browser: Option<Browser>,
    #[arg(long, requires = "select_smf")]
    pub preserve_position: bool,
    /// Show only the foreground browser; closing it saves and exits selection mode.
    #[arg(long, requires = "select_smf")]
    pub browser_only: bool,
    /// Initial patch search from an SMF program name; retry words as OR if empty.
    #[arg(long, requires = "select_smf")]
    pub patch_filter: Option<String>,
    /// Read and return the exact patch browser filters in this JSON sidecar.
    #[arg(long, requires = "select_smf")]
    pub patch_filter_file: Option<std::path::PathBuf>,
    /// Read and return the effect browser Category / Kind / cursor in this JSON sidecar.
    #[arg(long, requires = "select_smf")]
    pub effect_filter_file: Option<std::path::PathBuf>,
    /// Audition all tracks in this SMF through the selected instrument.
    #[arg(long, requires = "select_smf")]
    pub preview_smf: Option<std::path::PathBuf>,
    /// Start selection preview at this position in seconds.
    #[arg(long, requires = "select_smf", value_parser = seconds)]
    pub start_seconds: Option<f64>,
    /// Keep the whole-song loop length when the selected track ends earlier.
    #[arg(long, requires = "select_smf", value_parser = seconds)]
    pub loop_end_seconds: Option<f64>,
}

fn seconds(value: &str) -> Result<f64, String> {
    let value: f64 = value.parse().map_err(|_| "Expected seconds")?;
    if !value.is_finite() || value < 0.0 {
        return Err("Seconds must be finite and nonnegative".into());
    }
    Ok(value)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Browser {
    Patch,
    Effect,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_program_name_filter_only_in_selection_mode() {
        let cli = Cli::try_parse_from([
            "app",
            "--select-smf",
            "X:/one.mid",
            "--preserve-position",
            "--patch-filter",
            "church organ",
        ])
        .unwrap();
        assert_eq!(cli.patch_filter.as_deref(), Some("church organ"));
        assert!(Cli::try_parse_from(["app", "--patch-filter", "church organ"]).is_err());
    }

    #[test]
    fn selection_arguments_are_explicit_and_validated() {
        assert!(Cli::try_parse_from([
            "app",
            "--select-smf",
            "X:/one.mid",
            "--browser",
            "effect",
            "--preserve-position",
            "--browser-only"
        ])
        .is_ok());
        for args in [
            vec!["app", "--select-smf", "X:/one.mid"],
            vec!["app", "--browser", "effect"],
            vec!["app", "--preserve-position"],
            vec!["app", "--browser-only"],
            vec![
                "app",
                "--select-smf",
                "X:/one.mid",
                "--browser",
                "invalid",
                "--preserve-position",
            ],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn resident_mode_takes_a_name_and_excludes_file_selection() {
        let cli = Cli::try_parse_from(["app", "--resident", "editor-42"]).unwrap();
        assert_eq!(cli.resident.as_deref(), Some("editor-42"));
        assert!(Cli::try_parse_from(["app", "--resident"]).is_err());
        assert!(Cli::try_parse_from([
            "app",
            "--resident",
            "editor-42",
            "--select-smf",
            "X:/one.mid",
            "--preserve-position"
        ])
        .is_err());
    }

    #[test]
    fn accepts_gui_invocation() {
        assert!(Cli::try_parse_from(["app"]).is_ok());
    }

    #[test]
    fn validates_preview_timing_options() {
        let parsed = Cli::try_parse_from([
            "app",
            "--select-smf",
            "X:/one.mid",
            "--preserve-position",
            "--start-seconds",
            "12.5",
            "--loop-end-seconds",
            "60",
        ])
        .unwrap();
        assert_eq!(parsed.start_seconds, Some(12.5));
        assert_eq!(parsed.loop_end_seconds, Some(60.0));
        for value in ["NaN", "inf", "-1", "text"] {
            assert!(Cli::try_parse_from([
                "app",
                "--select-smf",
                "X:/one.mid",
                "--preserve-position",
                "--start-seconds",
                value
            ])
            .is_err());
        }
        assert!(Cli::try_parse_from(["app", "--start-seconds", "1"]).is_err());
        assert!(Cli::try_parse_from(["app", "--loop-end-seconds", "2"]).is_err());
    }

    #[test]
    fn rejects_invalid_invocations_instead_of_starting_gui() {
        for args in [
            vec!["app", "unknown"],
            vec!["app", "check"],
            vec!["app", "update"],
            vec!["app", "update", "extra"],
            vec!["app", "diagnose-scope"],
            vec!["app", "replay-scope", "--input", "X:/captures/one"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
        assert_eq!(
            Cli::try_parse_from(["app", "--help"]).unwrap_err().kind(),
            clap::error::ErrorKind::DisplayHelp
        );
    }
}
