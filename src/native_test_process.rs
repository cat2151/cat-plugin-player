//! App startup uses process-lifetime native/COM state; each case owns one process.
use std::{
    fs::{self, File},
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) const ROOT: &str = "CAT_NATIVE_TEST_ROOT";
const CHILD: &str = "CAT_NATIVE_TEST_CASE";

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Returns true in the parent after exactly this case passed in its child.
/// Call before loading the DLL or constructing an App; keep assertions in the case.
pub(crate) fn isolated() -> bool {
    let thread = std::thread::current();
    let name = thread
        .name()
        .expect("native fixture needs a named test thread");
    if let Some(child) = std::env::var_os(CHILD) {
        assert_eq!(child, name, "refusing recursive native test launch");
        return false;
    }
    let result = run(name, Duration::from_secs(120));
    assert!(
        result.passed(),
        "native case {name}: status={}, timeout={}\n{}\n{}",
        result.status,
        result.timed_out,
        result.stdout,
        result.stderr
    );
    true
}

struct Result {
    status: ExitStatus,
    timed_out: bool,
    stdout: String,
    stderr: String,
}

impl Result {
    fn passed(&self) -> bool {
        !self.timed_out
            && self.status.success()
            && self
                .stdout
                .lines()
                .filter(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"))
                .count()
                == 1
    }
}

fn run(name: &str, timeout: Duration) -> Result {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = Fixture(
        std::env::temp_dir().join(format!("cat-native-test-{}-{nonce}", std::process::id())),
    );
    fs::create_dir(&root.0).unwrap();
    let stdout = root.0.join("stdout.log");
    let stderr = root.0.join("stderr.log");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([name, "--exact", "--nocapture"])
        .env(CHILD, name)
        .env(ROOT, &root.0)
        .stdin(Stdio::null())
        .stdout(File::create(&stdout).unwrap())
        .stderr(File::create(&stderr).unwrap())
        .spawn()
        .unwrap();
    let started = Instant::now();
    let (status, timed_out) = loop {
        if started.elapsed() >= timeout {
            let _ = child.kill();
            break (child.wait().unwrap(), true);
        }
        if let Some(status) = child.try_wait().unwrap() {
            break (status, false);
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Result {
        status,
        timed_out,
        stdout: String::from_utf8_lossy(&fs::read(stdout).unwrap()).into_owned(),
        stderr: String::from_utf8_lossy(&fs::read(stderr).unwrap()).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_successful_zero_test_child_is_rejected() {
        let result = run(
            "native_test_case_that_does_not_exist",
            Duration::from_secs(10),
        );
        assert!(result.status.success());
        assert!(
            !result.passed(),
            "zero tests must not count as a native case pass"
        );
    }

    #[test]
    fn a_child_exceeding_its_deadline_is_killed_and_reaped() {
        let result = run(
            "plugin_scan::tests::cached_catalog_is_ready_without_loading_plugins_and_unchanged_refresh_keeps_generation",
            Duration::ZERO,
        );
        assert!(result.timed_out);
        assert!(!result.passed());
    }
}
