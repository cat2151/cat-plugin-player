//! Parse commands before initializing audio, plugins, or the GUI.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about = "オーディオプラグインを手軽に演奏するアプリ")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// ビルド時のコミットと GitHub の main を比較する
    Check,
    /// GitHub から最新版をビルドしてインストールする
    Update,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_gui_and_update_commands() {
        assert_eq!(Cli::try_parse_from(["app"]).unwrap().command, None);
        for (argument, expected) in [("check", Command::Check), ("update", Command::Update)] {
            assert_eq!(
                Cli::try_parse_from(["app", argument]).unwrap().command,
                Some(expected)
            );
        }
    }

    #[test]
    fn rejects_invalid_invocations_instead_of_starting_gui() {
        for args in [
            vec!["app", "unknown"],
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
