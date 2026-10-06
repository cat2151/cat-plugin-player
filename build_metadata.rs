//! Git metadata for update checks, including worktrees and packed refs.

use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_owned())
}

pub fn emit(root: &Path) {
    let hash = git(root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=GIT_COMMIT_HASH={hash}");

    let mut refs = vec!["HEAD".to_string(), "packed-refs".to_string()];
    if let Some(reference) = git(root, &["symbolic-ref", "-q", "HEAD"]) {
        refs.push(reference);
    }
    for reference in refs {
        if let Some(path) = git(root, &["rev-parse", "--git-path", &reference]) {
            let path = root.join(path);
            // Git may have no packed-refs, or the branch may exist only there.
            // Watching a missing file makes Cargo rerun this script every time.
            // HEAD and the existing loose/packed ref still track commit changes.
            if path.is_file() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
