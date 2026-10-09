//! Parse commands before initializing audio, plugins, or the GUI.

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about = "オーディオプラグインを手軽に演奏するアプリ")]
pub struct Cli {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_gui_invocation() {
        assert!(Cli::try_parse_from(["app"]).is_ok());
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
