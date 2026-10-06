//! Update commands shared with clap-mml-render-tui via cat-self-update-lib.

use crate::config;
use std::path::{Path, PathBuf};

const OWNER: &str = "cat2151";
const REPO: &str = "cat-plugin-player";
const BRANCH: &str = "main";

fn validate_hash(hash: &str) -> Result<&str, String> {
    let hash = hash.trim();
    if hash.len() != 40 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(
            "このビルドでは commit hash を取得できないため check を実行できません。git clone した作業ツリーからビルドし直してください。".into(),
        );
    }
    Ok(hash)
}

pub fn check() -> Result<(), String> {
    let hash = validate_hash(env!("GIT_COMMIT_HASH"))?;
    let result = cat_self_update_lib::check_remote_commit(OWNER, REPO, BRANCH, hash)
        .map_err(|e| format!("アップデート確認に失敗しました: {e}"))?;
    println!("{result}");
    Ok(())
}

pub fn update() -> Result<(), String> {
    let config_path = config::path()?;
    let config = config::Config::load(&config_path)
        .map_err(|e| format!("設定を読み込めません ({}): {e}", config_path.display()))?;
    // cargo install builds a new checkout, where ../uapmd no longer points at
    // the original dependency. Give the helper an absolute path to inherit.
    let uapmd = select_uapmd_dir(
        std::env::var_os("UAPMD_DIR").map(PathBuf::from),
        config.build.uapmd_dir,
        &config_path,
        Path::new(env!("BUILD_UAPMD_DIR")),
    );
    let hint = format!(
        "UAPMD_DIR 環境変数または {} の [build] uapmd_dir に UAPMD のソースの場所を指定してください",
        config_path.display()
    );
    if uapmd.as_os_str().is_empty() {
        return Err(hint);
    }
    let uapmd = uapmd
        .canonicalize()
        .map_err(|e| format!("{hint} ({}): {e}", uapmd.display()))?;
    if !uapmd.join("source/CMakeLists.txt").is_file() {
        return Err(format!(
            "{hint} ({}): source/CMakeLists.txt がありません",
            uapmd.display()
        ));
    }
    // Called before starting any application threads.
    std::env::set_var("UAPMD_DIR", uapmd);
    println!("アップデートを開始します...");
    cat_self_update_lib::self_update(OWNER, REPO, &[])
        .map_err(|e| format!("アップデート開始に失敗しました: {e}"))?;
    println!("バックグラウンドで cargo install を開始しました。完了後に cat-plugin-player を手動で起動してください。");
    Ok(())
}

fn select_uapmd_dir(
    environment: Option<PathBuf>,
    configured: Option<PathBuf>,
    config_path: &Path,
    built: &Path,
) -> PathBuf {
    environment.unwrap_or_else(|| match configured {
        Some(path) if !path.as_os_str().is_empty() && path.is_relative() => {
            config_path.parent().unwrap_or(Path::new(".")).join(path)
        }
        Some(path) => path,
        None => built.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_dependency_uses_env_config_then_build_and_resolves_relative_config() {
        let config_path = Path::new("X:/settings/config.toml");
        let built = Path::new("X:/original/uapmd");
        let configured = Some(PathBuf::from("X:/configured/UAPMD 日本語"));
        assert_eq!(select_uapmd_dir(None, None, config_path, built), built);
        assert_eq!(
            select_uapmd_dir(None, configured.clone(), config_path, built),
            configured.clone().unwrap()
        );
        assert_eq!(
            select_uapmd_dir(Some("X:/env/uapmd".into()), configured, config_path, built),
            Path::new("X:/env/uapmd")
        );
        assert_eq!(
            select_uapmd_dir(None, Some("../uapmd".into()), config_path, built),
            Path::new("X:/settings/../uapmd")
        );
        assert!(select_uapmd_dir(None, Some("".into()), config_path, built)
            .as_os_str()
            .is_empty());
        let config: config::Config =
            toml::from_str("[build]\nUAPMD_DIR = 'X:\\dependencies\\uapmd'\n").unwrap();
        assert_eq!(
            config.build.uapmd_dir.unwrap(),
            Path::new("X:/dependencies/uapmd")
        );
    }

    #[test]
    fn check_rejects_missing_or_malformed_build_metadata() {
        for hash in [
            "unknown",
            "",
            "abc",
            "0123456789abcdef0123456789abcdef0123456z",
        ] {
            assert!(validate_hash(hash).is_err());
        }
        let hash = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(validate_hash(&format!("{hash}\n")).unwrap(), hash);
    }
}
